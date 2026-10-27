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
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    static object ReceiverSample(Type type, int variant)
    {
        if (type.IsSZArray) {
            if (variant == 0) return null;
            Array array = Array.CreateInstance(type.GetElementType(), variant == 1 ? 0 : 2);
            for (int i = 0; i < array.Length; i++) array.SetValue(ReceiverSample(type.GetElementType(), i), i);
            return array;
        }
        object result = RuntimeHelpers.GetUninitializedObject(type);
        foreach (FieldInfo member in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
        {
            object value = member.FieldType == typeof(int) ? (variant == 0 ? int.MinValue : 7)
                : member.FieldType == typeof(long) ? (variant == 0 ? long.MinValue : 3L)
                : member.FieldType == typeof(object) ? (variant == 0 ? "owned" : new object())
                : ReceiverSample(member.FieldType, variant);
            WriteState(member, result, value);
        }
        return result;
    }

    static object ReceiverArgument(Type type, int variant) => type.IsByRef ? ReceiverArgument(type.GetElementType(), variant) : type == typeof(int)
        ? (variant == 0 ? int.MinValue : 11)
        : type == typeof(long) ? (variant == 0 ? long.MaxValue : -5L) : ReceiverSample(type, variant);

    static object Box(Type type, object value)
    {
        object box = RuntimeHelpers.GetUninitializedObject(type);
        foreach (FieldInfo member in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
            WriteState(member, box, ReadState(member, value));
        return box;
    }

    // A value-type instance method receives a managed pointer into the boxed
    // storage, so the recovered and original bodies mutate the same slot.
    static Func<object, object[], object> ReceiverCall(MethodInfo method, bool recovered, bool structReceiver)
    {
        var body = new DynamicMethod((recovered ? "Recovered_" : "Owned_") + method.Name, typeof(object),
            new[] { typeof(object), typeof(object[]) }, typeof(GenericReferenceRoundtrip).Module, true);
        var il = body.GetILGenerator();
        ParameterInfo[] parameters = method.GetParameters();
        bool instance = !method.IsStatic;
        if (instance)
        {
            il.Emit(OpCodes.Ldarg_0);
            il.Emit(method.DeclaringType.IsValueType ? OpCodes.Unbox : OpCodes.Castclass, method.DeclaringType);
        }
        else if (structReceiver)
        {
            // An instance method lowered to a static carrier passes the receiver
            // as an explicit managed pointer to the same boxed storage.
            il.Emit(OpCodes.Ldarg_0);
            il.Emit(OpCodes.Unbox, method.GetParameters()[0].ParameterType.GetElementType());
        }
        for (int index = structReceiver ? 1 : 0; index < parameters.Length; index++)
        {
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(OpCodes.Ldc_I4, index - (structReceiver ? 1 : 0));
            il.Emit(OpCodes.Ldelem_Ref);
            Type type = parameters[index].ParameterType;
            if (type.IsByRef && type.GetElementType().IsValueType)
                il.Emit(OpCodes.Unbox, type.GetElementType());
            else
                il.Emit(type.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, type);
        }
        il.Emit(OpCodes.Call, method);
        if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
        else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
        il.Emit(OpCodes.Ret);
        return (Func<object, object[], object>)body.CreateDelegate(typeof(Func<object, object[], object>));
    }

    static void ReceiverMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("StructReceiverCases.Point") == null)
            throw new Exception("Expected owned struct receiver fixture.");
        Type point = fixture.GetType("StructReceiverCases.Point");
        Type line = fixture.GetType("StructReceiverCases.Line");
        Type ops = fixture.GetType("StructReceiverCases.Ops");
        if (args[1] == "--receiver-plan")
        {
            using var json = JsonDocument.Parse(File.ReadAllText(args[2]));
            foreach (var item in json.RootElement.EnumerateArray())
            {
                string alias = item.GetProperty("alias").GetString();
                if (!Regex.IsMatch(alias, "^__cil_(value|ref)_[0-9a-f]{40}$")) throw new Exception("Invalid value alias.");
                byte[] bytes = Convert.FromHexString(item.GetProperty("bytes").GetString());
                int offset = 0;
                Type type = Decode(bytes, ref offset);
                Type valueType = type.IsSZArray ? type.GetElementType() : type;
                if (offset != bytes.Length || !valueType.IsValueType || valueType.IsEnum || valueType.IsByRefLike
                    || valueType.Assembly != fixture || (valueType != point && valueType != line)
                    || type.IsSZArray != alias.StartsWith("__cil_ref_", StringComparison.Ordinal))
                    throw new Exception("Owned receiver type alias disagrees with runtime metadata.");
                Console.WriteLine("using " + alias + " = " + TypeSource(type) + ";");
            }
            return;
        }
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
        FieldInfo[] statics = fixture.GetTypes().Where(t => !t.ContainsGenericParameters && !t.IsByRefLike)
            .SelectMany(t => t.GetFields(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly))
            .Where(f => !f.IsLiteral && !f.IsInitOnly).ToArray();
        MethodInfo[] selected = new[] { point, line, ops }.SelectMany(t => t.GetMethods(
                BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance | BindingFlags.DeclaredOnly))
            .Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal) || m.Name == "UnsupportedByRef" || m.Name == "UnsupportedLocalEscape" || m.Name == "UnsupportedBox" || m.Name == "UnsupportedArray")
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned receiver selection is empty.");
        Type carrier = generated.GetType("StructReceiverCases.Recovered")
            ?? throw new Exception("Recovered receiver carrier missing.");

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered receiver method: " + original.Name);
            ParameterInfo[] parameters = original.GetParameters();
            Type[] expected = original.IsStatic
                ? parameters.Select(p => p.ParameterType).ToArray()
                : parameters.Select(p => p.ParameterType)
                    .Prepend(original.DeclaringType.MakeByRefType()).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(expected))
                throw new Exception("Recovered receiver signature differs: " + original.Name
                    + " lowered=(" + lowered.ReturnType + " " + string.Join(",", lowered.GetParameters().Select(p => p.ParameterType.ToString())) + ")"
                    + " expected=(" + original.ReturnType + " " + string.Join(",", expected.Select(t => t.ToString())) + ")");

            for (int variant = 0; variant < (original.Name == "UnsupportedArray" ? 3 : 2); variant++)
            {
                object receiver = original.IsStatic ? null
                    : Box(original.DeclaringType, ReceiverSample(original.DeclaringType, variant));
                object[] arguments = parameters.Select((p, i) => ReceiverArgument(p.ParameterType, original.Name == "UnsupportedArray" ? variant : (variant + i) % 2)).ToArray();
                object[] initialStatics = statics.Select(f => ReadState(f, null)).ToArray();
                var graph = CaptureGraph(arguments.Prepend(receiver).Concat(initialStatics));
                void Restore()
                {
                    foreach (State slot in graph) slot.Write(slot.Initial);
                    for (int index = 0; index < statics.Length; index++) WriteState(statics[index], null, initialStatics[index]);
                }
                Outcome before = Capture(() => ReceiverCall(original, false, false)(receiver, arguments), graph);
                object[] beforeStatics = statics.Select(f => ReadState(f, null)).ToArray();
                Restore();
                Outcome after = Capture(() => ReceiverCall(lowered, true, !original.IsStatic)(receiver, arguments), graph);
                object[] afterStatics = statics.Select(f => ReadState(f, null)).ToArray();
                Restore();
                void EqualOrThrow(string kind)
                {
                    if (before.Error != after.Error || !Same(before.Value, after.Value)
                        || !beforeStatics.Zip(afterStatics).All(v => Same(v.First, v.Second))
                        || !before.State.Zip(after.State).All(pair => pair.First.Length == pair.Second.Length
                            && pair.First.Zip(pair.Second).All(v => Same(v.First, v.Second)))
                        || !before.Enumeration.Zip(after.Enumeration).All(v => Same(v.First, v.Second)))
                        throw new Exception("Recovered struct receiver behavior differs: " + original.Name + " (" + kind + ")");
                }
                if (original.Name == "UnsupportedArray" && before.Error != (variant == 0 ? typeof(NullReferenceException) : variant == 1 ? typeof(IndexOutOfRangeException) : null))
                    throw new Exception("Owned array receiver expectation differs.");
                if (original.Name != "UnsupportedArray" && before.Error != null && before.Error != typeof(NullReferenceException))
                    throw new Exception("Owned receiver case faulted: " + original.Name);
                EqualOrThrow("variant " + variant);
                checks++;
            }
        }
        Console.WriteLine($"Struct receiver behavior: {checks} comparisons, {selected.Length} recovered methods; managed-location writes, value-argument copies, nested field reads, static state and exceptions match.");
    }
}
