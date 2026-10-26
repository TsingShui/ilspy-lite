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
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.Loader;

// Execute only owned fixtures and recovered source. No third-party corpus input is accepted by the driver.
internal static class ObjectRoundtrip
{
    record Outcome(object Value, Type Error, object[] Fields, object[] Statics, Array[] Arrays);
    static readonly Dictionary<string, Func<object, object[], object>> Invokers = new();
    static readonly HashSet<string> Covered = new();
    static MethodInfo[] originals;
    static Type recovered;
    static Type cell;
    static FieldInfo field;
    static FieldInfo global;
    static FieldInfo[] statics;
    static int checks;

    static Func<object, object[], object> OriginalCall(MethodInfo method)
    {
        if (Invokers.TryGetValue(method.Name, out var existing)) return existing;
        var body = new DynamicMethod("Owned_" + method.Name, typeof(object),
            new[] { typeof(object), typeof(object[]) }, typeof(ObjectRoundtrip).Module, true);
        var il = body.GetILGenerator();
        if (!method.IsStatic)
        {
            il.Emit(OpCodes.Ldarg_0);
            il.Emit(OpCodes.Castclass, method.DeclaringType);
        }
        var parameters = method.GetParameters();
        for (int index = 0; index < parameters.Length; index++)
        {
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(OpCodes.Ldc_I4, index);
            il.Emit(OpCodes.Ldelem_Ref);
            Type type = parameters[index].ParameterType;
            il.Emit(type.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, type);
        }
        // IL call keeps an instance method's original null-receiver behavior.
        il.Emit(OpCodes.Call, method);
        if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
        else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
        il.Emit(OpCodes.Ret);
        var invoke = (Func<object, object[], object>)body.CreateDelegate(typeof(Func<object, object[], object>));
        Invokers.Add(method.Name, invoke);
        return invoke;
    }

    static Outcome Capture(Func<object> action, object[] cells, Array[] arrays)
    {
        object value = null;
        Type error = null;
        try
        {
            value = action();
        }
        catch (Exception exception)
        {
            if (exception is TargetInvocationException wrapped) exception = wrapped.InnerException;
            error = exception.GetType();
        }
        return new Outcome(value, error, cells.Select(c => field.GetValue(c)).ToArray(),
            statics.Select(f => f.GetValue(null)).ToArray(), arrays.Select(a => (Array)a.Clone()).ToArray());
    }

    static void Check(string name, object receiver, object[] args, object expected, Type expectedError = null)
    {
        MethodInfo original = originals.Single(m => m.Name == name);
        MethodInfo generated = recovered.GetMethod(name, BindingFlags.Public | BindingFlags.Static)
            ?? throw new Exception("Missing recovered method: " + name);
        object[] initialStatics = statics.Select(f => f.GetValue(null)).ToArray();
        object[] cells = args.Prepend(receiver).Concat(initialStatics).Where(v => v != null && v.GetType() == cell)
            .Distinct(ReferenceEqualityComparer.Instance).ToArray();
        object[] fields = cells.Select(c => field.GetValue(c)).ToArray();
        Array[] arrays = args.Concat(fields).Concat(initialStatics).OfType<Array>()
            .Distinct(ReferenceEqualityComparer.Instance).Cast<Array>().ToArray();
        Array[] initialArrays = arrays.Select(a => (Array)a.Clone()).ToArray();
        void Restore()
        {
            for (int index = 0; index < cells.Length; index++) field.SetValue(cells[index], fields[index]);
            for (int index = 0; index < statics.Length; index++) statics[index].SetValue(null, initialStatics[index]);
            for (int index = 0; index < arrays.Length; index++) Array.Copy(initialArrays[index], arrays[index], arrays[index].Length);
        }
        Outcome before = Capture(() => OriginalCall(original)(receiver, args), cells, arrays);
        Restore();
        object[] actualArgs = original.IsStatic ? args : args.Prepend(receiver).ToArray();
        Outcome after = Capture(() => generated.Invoke(null, actualArgs), cells, arrays);
        Restore();
        if (before.Error != expectedError || !ReferenceEquals(before.Value, expected))
            throw new Exception("Owned fixture expectation differs: " + name);
        bool same = before.Error == after.Error && ReferenceEquals(before.Value, after.Value)
            && before.Statics.Zip(after.Statics).All(pair => ReferenceEquals(pair.First, pair.Second))
            && before.Fields.Zip(after.Fields).All(pair => ReferenceEquals(pair.First, pair.Second))
            && before.Arrays.Zip(after.Arrays).All(pair => pair.First.Cast<object>().SequenceEqual(pair.Second.Cast<object>()));
        if (!same) throw new Exception("Recovered behavior differs: " + name);
        Covered.Add(name);
        checks++;
    }

    static object NewCell(object value)
    {
        object result = Activator.CreateInstance(cell);
        field.SetValue(result, value);
        return result;
    }

    static void Main(string[] args)
    {
        if (args.Length < 2 || args.Length > 3) throw new ArgumentException("Expected owned fixture and recovered library.");
        Assembly fixture = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[0]));
        if (fixture.GetType("ObjectCases.Ops") == null) throw new Exception("Not the owned object fixture.");
        cell = fixture.GetType("ObjectCases.Cell") ?? throw new Exception("Owned cell type missing.");
        field = cell.GetField("Value") ?? throw new Exception("Owned value field missing.");
        global = cell.GetField("Global") ?? throw new Exception("Owned static field missing.");
        statics = cell.GetFields(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        originals = fixture.GetTypes().SelectMany(t => t.GetMethods(BindingFlags.Public | BindingFlags.Static
            | BindingFlags.Instance | BindingFlags.DeclaredOnly)).Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal)).ToArray();
        recovered = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[1]))
            .GetType("Recovered") ?? throw new Exception("Recovered carrier missing.");
        object tag = new object();
        object other = new object();
        object seed = new object();
        global.SetValue(null, seed);
        if (args.Length == 3)
        {
            if (args[2] != "--forward-only") throw new ArgumentException("Unknown harness option.");
            Check("Forward", null, new[] { tag }, tag);
        }
        else
        {
            object[] values = { null, tag, "owned text", 42, new[] { 1, -2, int.MaxValue }, new uint[] { 0, uint.MaxValue }, NewCell(tag) };
            foreach (object value in values)
            {
                Check("Identity", null, new[] { value }, value);
                Check("Snapshot", null, new[] { value }, value);
                Check("Forward", null, new[] { value }, value);
                Check("AssignArgument", null, new[] { other, value }, value);
                Check("Choose", null, new object[] { true, value, other }, value);
                Check("Choose", null, new object[] { false, other, value }, value);
                Check("StaticExchange", null, new[] { value }, seed);
                object target = NewCell(value);
                Check("Read", target, Array.Empty<object>(), value);
                Check("Exchange", target, new[] { other }, value);
                Check("ReadField", null, new[] { target }, value);
                Check("ExchangeField", null, new[] { target, other }, value);
                Check("ReadAndClear", null, new[] { target }, value);
                Check("ExchangeField", null, new[] { target, value }, value);
            }
            Check("Null", null, Array.Empty<object>(), null);
            foreach (object value in new object[] { null, "text" }) Check("StringAsObject", null, new[] { value }, value);
            foreach (object value in new object[] { null, new[] { 1, 2 } }) Check("ArrayAsObject", null, new[] { value }, value);
            foreach (object value in new object[] { null, new uint[] { uint.MaxValue } }) Check("UnsignedArrayAsObject", null, new[] { value }, value);
            foreach (object value in new object[] { null, NewCell(other) }) Check("CellAsObject", null, new[] { value }, value);
            object oldText = "old text";
            cell.GetField("GlobalText").SetValue(null, oldText);
            foreach (object value in new object[] { null, "new text" }) Check("StaticStringExchange", null, new[] { value }, oldText);
            object oldCell = NewCell(tag);
            cell.GetField("GlobalCell").SetValue(null, oldCell);
            foreach (object value in new object[] { null, NewCell(other) }) Check("StaticCellExchange", null, new[] { value }, oldCell);
            object oldArray = new[] { 3, 4 };
            cell.GetField("GlobalArray").SetValue(null, oldArray);
            foreach (object value in new object[] { null, new[] { 5, 6 } }) Check("StaticArrayExchange", null, new[] { value }, oldArray);
            Check("Read", null, Array.Empty<object>(), null, typeof(NullReferenceException));
            Check("Exchange", null, new[] { other }, null, typeof(NullReferenceException));
            Check("ReadField", null, new object[] { null }, null, typeof(NullReferenceException));
            Check("ExchangeField", null, new[] { null, other }, null, typeof(NullReferenceException));
            Check("ReadAndClear", null, new object[] { null }, null, typeof(NullReferenceException));
            if (Covered.Count != 19 || originals.Length != 19) throw new Exception("Owned behavior coverage drift.");
        }
        Console.WriteLine($"Object behavior: {checks} cases, {Covered.Count} recovered methods; reference identity, fields, arrays, static state and exception types match.");
    }
}
