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
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    record State(Func<object[]> Read, Action<object[]> Write, object[] Initial, IEnumerable Sequence);
    record Outcome(object Value, Type Error, object[][] State, object[] Enumeration);
    static Assembly fixture;
    static Type cell;
    static FieldInfo field;
    static FieldInfo global;
    static Type recovered;
    static MethodInfo[] originals;
    static readonly Dictionary<MethodInfo, Func<object, object[], object>> Invokers = new();
    static readonly HashSet<string> Covered = new();
    static int checks;

    // Decode type signatures only, never IL or selected method implementations.
    static uint Unsigned(byte[] bytes, ref int position)
    {
        byte first = bytes[position++];
        if (first < 0x80) return first;
        if (first < 0xc0) return ((uint)(first & 0x3f) << 8) | bytes[position++];
        if (first >= 0xe0) throw new Exception("Bad compressed type integer.");
        return ((uint)(first & 0x1f) << 24) | ((uint)bytes[position++] << 16)
            | ((uint)bytes[position++] << 8) | bytes[position++];
    }

    static Type Nominal(byte[] bytes, ref int position, bool value)
    {
        uint encoded = Unsigned(bytes, ref position);
        uint rid = encoded >> 2;
        uint tag = encoded & 3;
        if (rid == 0 || rid > 0xffffff || tag > 1) throw new Exception("Bad signature type token.");
        int token = checked((int)((tag == 0 ? 0x02000000U : 0x01000000U) | rid));
        Type type = fixture.ManifestModule.ResolveType(token);
        if (type.IsValueType != value) throw new Exception("Signature nominal class/value mismatch.");
        return type;
    }

    static Type Decode(byte[] bytes, ref int position, Type[] typeArguments = null, Type[] methodArguments = null)
    {
        byte kind = bytes[position++];
        Type primitive = kind switch {
            2 => typeof(bool), 3 => typeof(char), 4 => typeof(sbyte), 5 => typeof(byte),
            6 => typeof(short), 7 => typeof(ushort), 8 => typeof(int), 9 => typeof(uint),
            10 => typeof(long), 11 => typeof(ulong), 12 => typeof(float), 13 => typeof(double),
            14 => typeof(string), 24 => typeof(nint), 25 => typeof(nuint), 28 => typeof(object),
            _ => null
        };
        if (primitive != null) return primitive;
        if (kind == 0x13 || kind == 0x1e)
        {
            uint index = Unsigned(bytes, ref position);
            Type[] positions = kind == 0x13 ? typeArguments : methodArguments;
            if (positions == null || index >= positions.Length) throw new Exception("Missing generic variable owner/position.");
            return positions[index];
        }
        if (kind == 0x11 || kind == 0x12) return Nominal(bytes, ref position, kind == 0x11);
        if (kind == 0x1d) return Decode(bytes, ref position, typeArguments, methodArguments).MakeArrayType();
        if (kind != 0x15) throw new Exception("Unsupported owned type signature.");
        byte category = bytes[position++];
        if (category != 0x11 && category != 0x12) throw new Exception("Bad generic category.");
        Type definition = Nominal(bytes, ref position, category == 0x11);
        int count = checked((int)Unsigned(bytes, ref position));
        if (!definition.IsGenericTypeDefinition || definition.GetGenericArguments().Length != count)
            throw new Exception("Generic signature arity mismatch.");
        var arguments = new Type[count];
        for (int index = 0; index < count; index++) arguments[index] = Decode(bytes, ref position, typeArguments, methodArguments);
        return definition.MakeGenericType(arguments);
    }

    static string TypeSource(Type type)
    {
        if (type.IsArray) return TypeSource(type.GetElementType()) + "[]";
        if (!type.IsGenericType) return "global::" + type.FullName.Replace('+', '.');
        Type[] arguments = type.GetGenericArguments();
        int cursor = 0;
        string name = Regex.Replace(type.GetGenericTypeDefinition().FullName.Replace('+', '.'),
            @"\x60(\d+)", match => {
                int count = int.Parse(match.Groups[1].Value);
                string result = "<" + string.Join(", ", arguments.Skip(cursor).Take(count).Select(TypeSource)) + ">";
                cursor += count;
                return result;
            });
        if (cursor != arguments.Length) throw new Exception("Nested type source arity mismatch.");
        return "global::" + name;
    }

    static void Aliases(string path)
    {
        using var json = JsonDocument.Parse(File.ReadAllText(path));
        foreach (var entry in json.RootElement.EnumerateArray())
        {
            string alias = entry.GetProperty("alias").GetString();
            if (!Regex.IsMatch(alias, "^__cil_ref_[0-9a-f]{40}$")) throw new Exception("Bad alias identifier.");
            byte[] bytes = Convert.FromHexString(entry.GetProperty("bytes").GetString());
            int position = 0;
            Type type = Decode(bytes, ref position);
            if (position != bytes.Length || type.IsValueType || !(type.IsConstructedGenericType || type.IsArray))
                throw new Exception("Incomplete or nonreference alias signature.");
            Console.WriteLine("using " + alias + " = " + TypeSource(type) + ";");
        }
    }

    static Func<object, object[], object> OriginalCall(MethodInfo method)
    {
        if (Invokers.TryGetValue(method, out var existing)) return existing;
        var body = new DynamicMethod("Owned_" + method.Name, typeof(object),
            new[] { typeof(object), typeof(object[]) }, typeof(GenericReferenceRoundtrip).Module, true);
        var il = body.GetILGenerator();
        if (!method.IsStatic) { il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Castclass, method.DeclaringType); }
        var parameters = method.GetParameters();
        for (int index = 0; index < parameters.Length; index++)
        {
            il.Emit(OpCodes.Ldarg_1); il.Emit(OpCodes.Ldc_I4, index); il.Emit(OpCodes.Ldelem_Ref);
            Type type = parameters[index].ParameterType;
            il.Emit(type.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, type);
        }
        il.Emit(OpCodes.Call, method);
        if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
        else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
        il.Emit(OpCodes.Ret);
        var invoke = (Func<object, object[], object>)body.CreateDelegate(typeof(Func<object, object[], object>));
        Invokers.Add(method, invoke);
        return invoke;
    }

    static object[] DictionaryValues(IDictionary dictionary)
    {
        var result = new List<object>();
        var iterator = dictionary.GetEnumerator();
        while (iterator.MoveNext()) { result.Add(iterator.Key); result.Add(iterator.Value); }
        return result.ToArray();
    }

    static List<State> CaptureGraph(IEnumerable<object> roots)
    {
        var result = new List<State>();
        var visited = new HashSet<object>(ReferenceEqualityComparer.Instance);
        var pending = new Queue<object>(roots);
        void Add(Func<object[]> read, Action<object[]> write, IEnumerable sequence = null)
        {
            object[] initial = read();
            result.Add(new State(read, write, initial, sequence));
            foreach (object value in initial) pending.Enqueue(value);
        }
        if (global != null) Add(() => new[] { ReadState(global, null) }, values => WriteState(global, null, values[0]));
        while (pending.Count != 0)
        {
            object target = pending.Dequeue();
            if (target == null || target is string || !visited.Add(target)) continue;
            if (target.GetType().IsValueType)
            {
                if (target.GetType().Assembly != fixture) continue;
                FieldInfo[] members = target.GetType().GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic);
                Add(() => members.Select(m => ReadState(m, target)).ToArray(),
                    values => { for (int index = 0; index < members.Length; index++) WriteState(members[index], target, values[index]); });
                continue;
            }
            if (target is Array array)
                Add(() => array.Cast<object>().ToArray(), values => {
                    for (int index = 0; index < values.Length; index++) array.SetValue(values[index], index);
                }, array);
            else if (target is IDictionary dictionary)
                Add(() => DictionaryValues(dictionary), values => {
                    dictionary.Clear();
                    for (int index = 0; index < values.Length; index += 2) dictionary.Add(values[index], values[index + 1]);
                }, dictionary);
            else if (target is IList list)
                Add(() => list.Cast<object>().ToArray(), values => {
                    list.Clear(); foreach (object value in values) list.Add(value);
                }, list);
            else if (target.GetType().Assembly == fixture)
                foreach (FieldInfo member in target.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance))
                    Add(() => new[] { ReadState(member, target) }, values => WriteState(member, target, values[0]));
            else if (target.GetType() != typeof(object)) throw new Exception("Unexpected owned state type: " + target.GetType());
        }
        return result;
    }

    static Outcome Capture(Func<object> action, List<State> state)
    {
        object value = null;
        Type error = null;
        var observers = state.Where(s => s.Sequence != null).Select(s => s.Sequence.GetEnumerator()).ToArray();
        try { value = action(); }
        catch (Exception exception)
        {
            if (exception is TargetInvocationException wrapped) exception = wrapped.InnerException;
            error = exception.GetType();
        }
        var enumeration = new List<object>();
        foreach (IEnumerator observer in observers)
        {
            try
            {
                bool moved = observer.MoveNext();
                enumeration.Add(null);
                enumeration.Add(moved);
                enumeration.Add(moved ? observer.Current : null);
            }
            catch (Exception exception)
            {
                enumeration.Add(exception.GetType());
                enumeration.Add(false);
                enumeration.Add(null);
            }
            if (observer is IDisposable disposable) disposable.Dispose();
        }
        return new Outcome(value, error, state.Select(s => s.Read()).ToArray(), enumeration.ToArray());
    }

    static bool Same(object left, object right)
    {
        if (ReferenceEquals(left, right)) return true;
        if (left == null || right == null || left.GetType() != right.GetType()) return false;
        Type type = left.GetType();
        if (type == typeof(double)) return BitConverter.DoubleToInt64Bits((double)left) == BitConverter.DoubleToInt64Bits((double)right);
        if (type == typeof(float)) return BitConverter.SingleToInt32Bits((float)left) == BitConverter.SingleToInt32Bits((float)right);
        if (type == typeof(decimal)) return decimal.GetBits((decimal)left).SequenceEqual(decimal.GetBits((decimal)right));
        if (type.IsPrimitive || type.IsEnum || type == typeof(nint) || type == typeof(nuint)) return left.Equals(right);
        if (!type.IsValueType) return false;
        return type.GetFields(BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic)
            .All(member => Same(ReadState(member, left), ReadState(member, right)));
    }

    static void Check(string name, object receiver, object[] arguments, object expected, Type expectedError = null)
    {
        MethodInfo original = originals.Single(m => m.Name == name);
        MethodInfo generated = recovered.GetMethod(name, BindingFlags.Public | BindingFlags.Static)
            ?? throw new Exception("Missing recovered method: " + name);
        Type[] expectedParameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
        if (!original.IsStatic) expectedParameters = expectedParameters.Prepend(original.DeclaringType).ToArray();
        if (generated.ReturnType != original.ReturnType
            || !generated.GetParameters().Select(p => p.ParameterType).SequenceEqual(expectedParameters))
            throw new Exception("Recovered structural signature differs: " + name);
        var state = CaptureGraph(arguments.Prepend(receiver));
        void Restore() { foreach (State slot in state) slot.Write(slot.Initial); }
        Outcome before = Capture(() => OriginalCall(original)(receiver, arguments), state);
        Restore();
        object[] actualArguments = original.IsStatic ? arguments : arguments.Prepend(receiver).ToArray();
        Outcome after = Capture(() => generated.Invoke(null, actualArguments), state);
        Restore();
        if (before.Error != expectedError || !ReferenceEquals(before.Value, expected))
            throw new Exception("Owned fixture expectation differs: " + name);
        if (before.Error != after.Error || !ReferenceEquals(before.Value, after.Value)
            || !before.Enumeration.Zip(after.Enumeration).All(values => Same(values.First, values.Second))
            || !before.State.Zip(after.State).All(pair => pair.First.Length == pair.Second.Length
                && pair.First.Zip(pair.Second).All(values => Same(values.First, values.Second))))
            throw new Exception("Recovered behavior differs: " + name);
        checks++; Covered.Add(name);
    }

    static object NewCell(object value)
    {
        object result = Activator.CreateInstance(cell);
        field.SetValue(result, value);
        return result;
    }

    static object NewBox(Type argument, object value)
    {
        Type type = fixture.GetType("GenericReferenceCases.Box`1").MakeGenericType(argument);
        object result = Activator.CreateInstance(type);
        type.GetField("Value").SetValue(result, value);
        return result;
    }

    static void ArrayCases(string[] args)
    {
        object first = new byte[] { 0, 1, byte.MaxValue };
        if (args.Length == 3)
        {
            if (args[2] != "--forward-only") throw new ArgumentException("Unknown harness option.");
            Check("Forward", null, new[] { first }, first);
        }
        else
        {
            object[] arrays = { null, Array.Empty<byte>(), first };
            foreach (object value in arrays)
            {
                Check("LocalIdentity", null, new[] { value }, value);
                Check("Forward", null, new[] { value }, value);
                Check("Snapshot", null, new[] { value }, value);
                Check("Upcast", null, new[] { value }, value);
                object target = NewCell(value);
                Check("ReadField", null, new[] { target }, value);
                foreach (object next in arrays)
                {
                    Check("Exchange", target, new[] { next }, value);
                    Check("ExchangeField", null, new[] { target, next }, value);
                    Check("Choose", null, new[] { (object)true, value, next }, value);
                    Check("Choose", null, new[] { (object)false, next, value }, value);
                }
                Check("Exchange", null, new[] { value }, null, typeof(NullReferenceException));
                Check("ExchangeField", null, new[] { null, value }, null, typeof(NullReferenceException));
            }
            Check("ReadField", null, new object[] { null }, null, typeof(NullReferenceException));
            Check("Null", null, Array.Empty<object>(), null);
            object tag = new object();
            var state = NewCell(first);
            var names = new[] { "StringIdentity", "ExternalIdentity", "ArrayArgument", "EnumIdentity",
                "CharIdentity", "FloatIdentity", "LongIdentity", "NativeIdentity", "UnsignedIdentity" };
            Type point = fixture.GetType("ArrayReferenceCases.Point");
            Array points = Array.CreateInstance(point, 2);
            object valuePoint = Activator.CreateInstance(point);
            point.GetField("X").SetValue(valuePoint, int.MinValue);
            point.GetField("Tag").SetValue(valuePoint, "owned value reference");
            points.SetValue(valuePoint, 0);
            Check("ValueArgument", null, new object[] { null }, null);
            Array emptyPoints = Array.CreateInstance(point, 0);
            Check("ValueArgument", null, new object[] { emptyPoints }, emptyPoints);
            Check("ValueArgument", null, new object[] { points }, points);
            Type kind = fixture.GetType("ArrayReferenceCases.Kind");
            Array enums = Array.CreateInstance(kind, 2);
            enums.SetValue(Enum.ToObject(kind, long.MaxValue), 1);
            Array[] samples = {
                new string[] { null, "owned" }, new object[] { null, tag, state, first },
                new List<int>[] { null, new(), new() { -1, int.MaxValue } }, enums,
                new[] { '\0', '\ud800', '\uffff' },
                new[] { 0.0, -0.0, double.PositiveInfinity, BitConverter.Int64BitsToDouble(unchecked((long)0x7ff8000000000001UL)) },
                new[] { long.MinValue, -1L, long.MaxValue }, new nint[] { nint.MinValue, 0, nint.MaxValue },
                new[] { 0UL, ulong.MaxValue }
            };
            for (int index = 0; index < samples.Length; index++)
            {
                Array sample = samples[index];
                Check(names[index], null, new object[] { null }, null);
                Array empty = Array.CreateInstance(sample.GetType().GetElementType(), 0);
                Check(names[index], null, new object[] { empty }, empty);
                Check(names[index], null, new object[] { sample }, sample);
            }
            object covariantText = new string[] { "owned", null };
            Check("ExternalIdentity", null, new[] { covariantText }, covariantText);
            Array covariantCells = Array.CreateInstance(cell, 1);
            covariantCells.SetValue(state, 0);
            Check("ExternalIdentity", null, new object[] { covariantCells }, covariantCells);
            object[] jagged = { null, Array.Empty<byte[]>(), new byte[][] { null, (byte[])first, (byte[])first } };
            foreach (object value in jagged)
            {
                Check("NestedIdentity", null, new[] { value }, value);
                foreach (object before in jagged)
                {
                    global.SetValue(null, before);
                    Check("StaticExchange", null, new[] { value }, before);
                }
            }
            if (Covered.Count != 21 || originals.Length != 21) throw new Exception("Owned array coverage drift.");
        }
        Console.WriteLine($"Array reference behavior: {checks} cases, {Covered.Count} recovered methods; exact types, identity, reachable state, bit patterns and exception types match.");
    }

    static void Main(string[] args)
    {
        if (args.Length < 2 || args.Length > 4) throw new ArgumentException("Expected owned fixture and recovered library.");
        fixture = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[0]));
        if (args[1] == "--generic-declaring-check")
        {
            GenericDeclaringMain(args);
            return;
        }
        if (args[1] == "--generic-value-check")
        {
            GenericValueMain(args);
            return;
        }
        if (args[1] == "--external-allocation-check")
        {
            ExternalAllocationMain(args);
            return;
        }
        if (args[1] == "--external-value-check")
        {
            ExternalValueMain(args);
            return;
        }
        if (args[1] == "--external-call-check")
        {
            ExternalCallMain(args);
            return;
        }
        if (args[1] == "--external-enum-check")
        {
            ExternalEnumMain(args);
            return;
        }
        if (args[1] == "--cast-check")
        {
            CastMain(args);
            return;
        }
        if (args[1] == "--static-ctor-check")
        {
            StaticConstructorMain(args);
            return;
        }
        if (args[1] == "--equality-check")
        {
            EqualityMain(args);
            return;
        }
        if (args[1] == "--newobj-plan" || args[1] == "--newobj-check")
        {
            NewObjectMain(args);
            return;
        }
        if (args[1] == "--virtual-plan" || args[1] == "--virtual-check")
        {
            VirtualMain(args);
            return;
        }
        if (args[1] == "--receiver-plan" || args[1] == "--receiver-check")
        {
            ReceiverMain(args);
            return;
        }
        if (args[1] == "--struct-plan" || args[1] == "--struct-check" || args[1] == "--struct-forward")
        {
            StructMain(args);
            return;
        }
        if (args[1] == "--enum-plan" || args[1] == "--enum-check" || args[1] == "--enum-forward")
        {
            EnumMain(args);
            return;
        }
        if (args[1] == "--scoped-plan" || args[1] == "--scoped-check")
        {
            ScopedMain(args);
            return;
        }
        string root = fixture.GetType("ArrayReferenceCases.Cell") != null ? "ArrayReferenceCases" : "GenericReferenceCases";
        cell = fixture.GetType(root + ".Cell") ?? throw new Exception("Not an owned reference fixture.");
        field = cell.GetField("Values") ?? throw new Exception("Owned generic field missing.");
        global = cell.GetField("Cache") ?? throw new Exception("Owned generic static missing.");
        if (args[1] == "--aliases")
        {
            if (args.Length != 3) throw new ArgumentException("Expected owned signature directive file.");
            Aliases(args[2]);
            return;
        }
        originals = fixture.GetTypes().SelectMany(t => t.GetMethods(BindingFlags.Public | BindingFlags.Static
            | BindingFlags.Instance | BindingFlags.DeclaredOnly)).Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal)).ToArray();
        recovered = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[1])).GetType("Recovered")
            ?? throw new Exception("Recovered carrier missing.");
        if (root == "ArrayReferenceCases") { ArrayCases(args); return; }
        object firstBox = NewBox(typeof(int), 17);
        if (args.Length == 3)
        {
            if (args[2] != "--forward-only") throw new ArgumentException("Unknown harness option.");
            Check("Forward", null, new[] { firstBox }, firstBox);
        }
        else
        {
            object[] boxes = { null, firstBox, NewBox(typeof(int), int.MinValue), NewBox(typeof(int), int.MaxValue) };
            foreach (object value in boxes)
            {
                Check("LocalIdentity", null, new[] { value }, value);
                Check("Forward", null, new[] { value }, value);
            }
            foreach (object value in new object[] { null, NewBox(typeof(string), null), NewBox(typeof(string), "owned") })
                Check("StringIdentity", null, new[] { value }, value);
            object[] lists = { null, new List<int>(), new List<int> { 1, -2, int.MinValue, int.MaxValue } };
            foreach (object value in lists)
            {
                Check("ExternalIdentity", null, new[] { value }, value);
                Check("Snapshot", null, new[] { value }, value);
                Check("Upcast", null, new[] { value }, value);
                object target = NewCell(value);
                Check("ReadField", null, new[] { target }, value);
                foreach (object next in lists)
                {
                    Check("Exchange", target, new[] { next }, value);
                    Check("ExchangeField", null, new[] { target, next }, value);
                    Check("Choose", null, new[] { (object)true, value, next }, value);
                    Check("Choose", null, new[] { (object)false, next, value }, value);
                }
                Check("Exchange", null, new[] { value }, null, typeof(NullReferenceException));
                Check("ExchangeField", null, new[] { null, value }, null, typeof(NullReferenceException));
            }
            Check("ReadField", null, new object[] { null }, null, typeof(NullReferenceException));
            Check("Null", null, Array.Empty<object>(), null);
            foreach (object value in new object[] { null, new List<int?>(), new List<int?> { null, 1, int.MaxValue } })
                Check("ValueArgument", null, new[] { value }, value);
            foreach (object value in new object[] { null, new List<int[]>(), new List<int[]> { null, new[] { -1, int.MaxValue } } })
                Check("ArrayArgument", null, new[] { value }, value);
            object[] dictionaries = { null, new Dictionary<string, List<int>>(),
                new Dictionary<string, List<int>> { ["owned"] = (List<int>)lists[2], ["empty"] = null } };
            foreach (object value in dictionaries)
            {
                Check("NestedIdentity", null, new[] { value }, value);
                foreach (object before in dictionaries)
                {
                    global.SetValue(null, before);
                    Check("StaticExchange", null, new[] { value }, before);
                }
            }
            if (Covered.Count != 15 || originals.Length != 15) throw new Exception("Owned generic coverage drift.");
        }
        Console.WriteLine($"Generic reference behavior: {checks} cases, {Covered.Count} recovered methods; exact types, reference identity, reachable fields/collections/arrays, static state and exception types match.");
    }
}
