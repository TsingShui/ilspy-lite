// Copyright (c) 2026 Tsingshui
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this
// software and associated documentation files (the "Software"), to deal in the Software
// without restriction, including without limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons
// to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or
// substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED,
// INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR
// PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
// OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

using System;
using System.IO;
using System.Collections.Generic;
using System.Reflection.Emit;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    static readonly Dictionary<FieldInfo, Func<object, object>> StateReaders = new();
    static readonly Dictionary<FieldInfo, Action<object, object>> StateWriters = new();

    // Exact field-leaf IL respects packed field alignment. It neither calls
    // selected methods nor reconstructs any recovered statements.
    static object ReadState(FieldInfo member, object target)
    {
        if (!StateReaders.TryGetValue(member, out var read))
        {
            var method = new DynamicMethod("ReadState", typeof(object), new[] { typeof(object) },
                typeof(GenericReferenceRoundtrip).Module, true);
            var il = method.GetILGenerator();
            if (!member.IsStatic)
            {
                il.Emit(OpCodes.Ldarg_0);
                il.Emit(member.DeclaringType.IsValueType ? OpCodes.Unbox : OpCodes.Castclass, member.DeclaringType);
            }
            il.Emit(member.IsStatic ? OpCodes.Ldsfld : OpCodes.Ldfld, member);
            if (member.FieldType.IsValueType) il.Emit(OpCodes.Box, member.FieldType);
            il.Emit(OpCodes.Ret);
            read = (Func<object, object>)method.CreateDelegate(typeof(Func<object, object>));
            StateReaders.Add(member, read);
        }
        return read(target);
    }

    static void WriteState(FieldInfo member, object target, object value)
    {
        if (member.IsInitOnly)
        {
            // Readonly fixture fields are assigned only during owned test-data setup.
            member.SetValue(target, value);
            return;
        }
        if (!StateWriters.TryGetValue(member, out var write))
        {
            var method = new DynamicMethod("WriteState", typeof(void), new[] { typeof(object), typeof(object) },
                typeof(GenericReferenceRoundtrip).Module, true);
            var il = method.GetILGenerator();
            if (!member.IsStatic)
            {
                il.Emit(OpCodes.Ldarg_0);
                il.Emit(member.DeclaringType.IsValueType ? OpCodes.Unbox : OpCodes.Castclass, member.DeclaringType);
            }
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(member.FieldType.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, member.FieldType);
            il.Emit(member.IsStatic ? OpCodes.Stsfld : OpCodes.Stfld, member);
            il.Emit(OpCodes.Ret);
            write = (Action<object, object>)method.CreateDelegate(typeof(Action<object, object>));
            StateWriters.Add(member, write);
        }
        write(target, value);
    }

    static object StructSample(Type type, int variant)
    {
        if (type == typeof(int)) return variant == 0 ? int.MinValue : int.MaxValue;
        if (type == typeof(byte)) return variant == 0 ? (byte)0 : byte.MaxValue;
        if (type == typeof(long)) return variant == 0 ? long.MinValue : long.MaxValue;
        if (type == typeof(double)) return variant == 0 ? -0.0 : BitConverter.Int64BitsToDouble(0x7ff8000000000123);
        if (type == typeof(string)) return variant == 0 ? "owned text" : "other text";
        if (type == typeof(object)) return variant == 0 ? "owned tag" : new object();
        if (type == typeof(object[])) return new object[] { "array tag", new object(), null };
        if (type.Assembly != fixture || !type.IsValueType) throw new Exception("Unexpected owned struct data type.");
        bool trace = Environment.GetEnvironmentVariable("ILSPY_OWNED_STRUCT_TRACE") == "1";
        if (trace) Console.Error.WriteLine($"sample {type.Name}, variant={variant}");
        object result = type.Name == "WithCtor" ? Activator.CreateInstance(type) : RuntimeHelpers.GetUninitializedObject(type);
        if (type.Name == "Union")
        {
            WriteState(type.GetField("Bits"), result, variant == 0 ? 0x80000000u : 0x7fc00123u);
        }
        else
            foreach (FieldInfo member in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
            {
                if (trace) Console.Error.WriteLine("  write " + member.Name);
                WriteState(member, result, StructSample(member.FieldType, variant));
            }
        return result;
    }

    static object WithNumber(int number)
    {
        Type pair = fixture.GetType("StructCopyCases.Pair");
        object value = RuntimeHelpers.GetUninitializedObject(pair);
        WriteState(pair.GetField("Number"), value, number);
        return value;
    }

    static void StructMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("StructCopyCases.PairOps") == null)
            throw new Exception("Expected owned struct fixture.");
        string[] names = { "Pair", "Nested", "Packed", "Union", "Frozen", "WithCtor", "Empty" };
        if (args[1] == "--struct-plan")
        {
            using var json = JsonDocument.Parse(File.ReadAllText(args[2]));
            int count = 0;
            foreach (var item in json.RootElement.EnumerateArray())
            {
                string alias = item.GetProperty("alias").GetString();
                if (!Regex.IsMatch(alias, "^__cil_value_[0-9a-f]{40}$")) throw new Exception("Invalid value alias.");
                byte[] bytes = Convert.FromHexString(item.GetProperty("bytes").GetString());
                if (bytes[0] != 0x11) throw new Exception("Not a nominal value signature.");
                int offset = 0;
                Type type = Decode(bytes, ref offset);
                if (offset != bytes.Length || !type.IsValueType || type.IsEnum || type.IsByRefLike
                    || type.ContainsGenericParameters || type.Assembly != fixture || !names.Contains(type.Name))
                    throw new Exception("Owned aggregate type alias disagrees with runtime metadata.");
                Console.WriteLine("using " + alias + " = " + TypeSource(type) + ";");
                count++;
            }
            if (count != names.Length) throw new Exception("Owned struct alias coverage drift.");
            return;
        }
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
        FieldInfo[] stateFields = fixture.GetTypes().Where(t => !t.ContainsGenericParameters && !t.IsByRefLike)
            .SelectMany(t => t.GetFields(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly))
            .Where(f => !f.IsLiteral && !f.IsInitOnly).ToArray();
        foreach (string name in names)
        {
            Type valueType = fixture.GetType("StructCopyCases." + name) ?? throw new Exception("Owned struct missing.");
            Type owner = fixture.GetType("StructCopyCases." + name + "Ops") ?? throw new Exception("Owned struct operations missing.");
            Type carrier = generated.GetType("Struct_" + name + ".Recovered") ?? throw new Exception("Recovered struct carrier missing.");
            field = owner.GetField("Value") ?? throw new Exception("Owned struct field missing.");
            global = owner.GetField("Global") ?? throw new Exception("Owned struct static missing.");
            MethodInfo[] methods = owner.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                .Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal) || m.Name == "UnsupportedDefault").ToArray();
            if (methods.Length != 10) throw new Exception("Owned struct method selection drift.");
            object first = StructSample(valueType, 0), second = StructSample(valueType, 1);
            object zero = RuntimeHelpers.GetUninitializedObject(valueType);
            object[] values = { zero, first, second };
            WriteState(global, null, first);

            void CheckStruct(string method, object receiver, object[] arguments, object expected, Type expectedError = null)
            {
                bool trace = Environment.GetEnvironmentVariable("ILSPY_OWNED_STRUCT_TRACE") == "1";
                if (trace) Console.Error.WriteLine($"struct case {checks}: {owner.Name}.{method}, nullReceiver={receiver == null}");
                MethodInfo original = methods.Single(m => m.Name == method);
                MethodInfo lowered = carrier.GetMethod(method, BindingFlags.Public | BindingFlags.Static)
                    ?? throw new Exception("Missing recovered struct method.");
                Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
                if (!original.IsStatic) parameters = parameters.Prepend(owner).ToArray();
                if (lowered.ReturnType != original.ReturnType
                    || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                    throw new Exception("Struct carrier signature differs.");
                object[] initialStatics = stateFields.Select(f => ReadState(f, null)).ToArray();
                var graph = CaptureGraph(arguments.Prepend(receiver).Concat(initialStatics));
                void Restore()
                {
                    foreach (State slot in graph) slot.Write(slot.Initial);
                    for (int index = 0; index < stateFields.Length; index++) WriteState(stateFields[index], null, initialStatics[index]);
                }
                if (trace) Console.Error.WriteLine("  original");
                Outcome before = Capture(() => OriginalCall(original)(receiver, arguments), graph);
                object[] beforeStatics = stateFields.Select(f => ReadState(f, null)).ToArray();
                Restore();
                if (trace) Console.Error.WriteLine("  recovered");
                Outcome after = Capture(() => lowered.Invoke(null, original.IsStatic ? arguments : arguments.Prepend(receiver).ToArray()), graph);
                object[] afterStatics = stateFields.Select(f => ReadState(f, null)).ToArray();
                Restore();
                if (before.Error != expectedError || !Same(before.Value, expected))
                    throw new Exception("Owned struct expectation differs: " + owner.Name + "." + method);
                if (before.Error != after.Error || !Same(before.Value, after.Value)
                    || !beforeStatics.Zip(afterStatics).All(v => Same(v.First, v.Second))
                    || !before.State.Zip(after.State).All(pair => pair.First.Length == pair.Second.Length
                        && pair.First.Zip(pair.Second).All(v => Same(v.First, v.Second)))
                    || !before.Enumeration.Zip(after.Enumeration).All(v => Same(v.First, v.Second)))
                    throw new Exception("Recovered struct behavior differs: " + owner.Name + "." + method);
                checks++;
            }

            CheckStruct("UnsupportedDefault", null, Array.Empty<object>(), zero);
            if (args[1] == "--struct-forward") CheckStruct("Forward", null, new[] { first }, first);
            else
            {
                foreach (object value in values)
                {
                    CheckStruct("Identity", null, new[] { value }, value);
                    CheckStruct("Forward", null, new[] { value }, value);
                    CheckStruct("StaticExchange", null, new[] { value }, first);
                    object receiver = Activator.CreateInstance(owner);
                    WriteState(field, receiver, value);
                    CheckStruct("Read", receiver, Array.Empty<object>(), value);
                    foreach (object next in values)
                    {
                        CheckStruct("Snapshot", null, new[] { value, next }, value);
                        CheckStruct("AssignArgument", null, new[] { value, next }, next);
                        CheckStruct("Choose", null, new[] { (object)true, value, next }, value);
                        CheckStruct("Choose", null, new[] { (object)false, value, next }, next);
                        CheckStruct("Exchange", receiver, new[] { next }, value);
                        CheckStruct("ExchangeField", null, new[] { receiver, next }, value);
                    }
                }
                CheckStruct("Read", null, Array.Empty<object>(), null, typeof(NullReferenceException));
                CheckStruct("Exchange", null, new[] { first }, null, typeof(NullReferenceException));
                CheckStruct("ExchangeField", null, new[] { null, first }, null, typeof(NullReferenceException));
            }
        }
        // Methods promoted out of the fallback set by initobj/ldflda admission.
        Type promoted = fixture.GetType("StructCopyCases.UnsupportedOps");
        Type promotedPair = fixture.GetType("StructCopyCases.Pair");
        Type promotedCarrier = generated.GetType("Struct_Unsupported.Recovered")
            ?? throw new Exception("Recovered promoted carrier missing.");
        object promotedBox = StructSample(promotedPair, 0);
        object promotedZero = RuntimeHelpers.GetUninitializedObject(promotedPair);
        foreach (var (name, arguments, expected) in new (string, object[], object)[] {
            ("UnsupportedMember", new[] { promotedBox }, ReadState(promotedPair.GetField("Number"), promotedBox)),
            ("UnsupportedNew", new object[] { 21 }, WithNumber(21)),
        })
        {
            MethodInfo original = promoted.GetMethod(name);
            MethodInfo lowered = promotedCarrier.GetMethod(name)
                ?? throw new Exception("Missing promoted carrier: " + name);
            object actual = lowered.Invoke(null, arguments);
            object reference = original.Invoke(null, arguments);
            if (!Same(reference, expected) || !Same(actual, expected))
                throw new Exception("Recovered promoted struct behavior differs: " + name);
            checks++;
        }
        {
            MethodInfo original = promoted.GetMethod("UnsupportedByRef") ?? throw new Exception("Owned byref copy missing.");
            MethodInfo lowered = promotedCarrier.GetMethod("UnsupportedByRef") ?? throw new Exception("Recovered byref copy missing.");
            Type byref = promotedPair.MakeByRefType();
            if (original.ReturnType != promotedPair || lowered.ReturnType != promotedPair
                || original.GetParameters().Single().ParameterType != byref || lowered.GetParameters().Single().ParameterType != byref)
                throw new Exception("Promoted byref copy signature differs.");
            Type cellType = typeof(ManagedArgument<>).MakeGenericType(promotedPair);
            FieldInfo slot = cellType.GetField("Value");
            foreach (object initial in new[] { promotedZero, promotedBox, StructSample(promotedPair, 1) })
            {
                object storage = Activator.CreateInstance(cellType);
                slot.SetValue(storage, initial);
                Outcome before = Capture(() => ManagedInvoker(original)(new[] { storage }), new List<State>());
                object beforeSlot = slot.GetValue(storage);
                slot.SetValue(storage, initial);
                Outcome after = Capture(() => ManagedInvoker(lowered)(new[] { storage }), new List<State>());
                if (before.Error != null || after.Error != before.Error || !Same(before.Value, initial)
                    || !Same(after.Value, initial) || !Same(beforeSlot, initial) || !Same(slot.GetValue(storage), initial))
                    throw new Exception("Recovered struct behavior differs: UnsupportedOps.UnsupportedByRef");
                checks++;
            }
        }
        Console.WriteLine($"Struct copy behavior: {checks} comparisons; seven exact aggregate types, scalar bits, embedded reference identity, snapshots, all public static state/constructor counts and exceptions match.");
    }
}
