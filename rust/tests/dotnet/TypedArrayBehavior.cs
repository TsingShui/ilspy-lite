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
using System.Runtime.Loader;
using System.Text.Json;

internal static partial class GenericReferenceRoundtrip
{
    static void TypedArrayPlan(string path)
    {
        using var json = JsonDocument.Parse(System.IO.File.ReadAllText(path));
        var results = new List<object>();
        int number = 0;
        foreach (var row in json.RootElement.EnumerateArray()) {
            int token = row.GetProperty("token").GetInt32();
            var definition = (MethodInfo)fixture.ManifestModule.ResolveMethod(token);
            bool genericOwner = definition.DeclaringType.IsGenericTypeDefinition;
            var arguments = definition.IsGenericMethodDefinition || genericOwner ? new[] { typeof(int), typeof(string), typeof(object),
                typeof(double), typeof(bool), typeof(int?), fixture.GetType("TypedArrayCases.Pair") ?? throw new Exception("Owned pair missing.") } : new[] { typeof(void) };
            for (int seed = 0; seed < arguments.Length; seed++) {
                var typeArgs = genericOwner ? new[] { arguments[seed] } : Type.EmptyTypes;
                var methodArgs = definition.IsGenericMethodDefinition ? new[] { arguments[(seed + typeArgs.Length) % arguments.Length] } : Type.EmptyTypes;
                var member = genericOwner ? definition.DeclaringType.MakeGenericType(typeArgs).GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
                    .Single(m => m.MetadataToken == token) : definition;
                var original = methodArgs.Length == 0 ? member : member.MakeGenericMethod(methodArgs);
                string header = "public static " + ValueCallType(original.ReturnType) + " " + original.Name + "("
                    + string.Join(", ", original.GetParameters().Select(p => ConstrainedParameter(p.ParameterType, p.Name))) + ")";
                var aliases = new List<string>();
                foreach (var alias in row.GetProperty("aliases").EnumerateArray()) {
                    byte[] bytes = Convert.FromHexString(alias.GetProperty("bytes").GetString());
                    int offset = 0;
                    Type type = Decode(bytes, ref offset, typeArgs, methodArgs);
                    if (offset != bytes.Length || type.ContainsGenericParameters) throw new Exception("Incomplete typed vector alias.");
                    aliases.Add("using " + alias.GetProperty("alias").GetString() + " = " + ValueCallType(type) + ";");
                }
                var helpers = new List<string>();
                string typeContext = "new System.Type[]{" + string.Join(", ", typeArgs.Select(t => "typeof(" + ValueCallType(t) + ")")) + "}";
                foreach (var allocation in row.GetProperty("allocations").EnumerateArray()) {
                    int elementToken = allocation.GetInt32();
                    Type element = fixture.ManifestModule.ResolveType(elementToken, typeArgs, methodArgs);
                    if (element.ContainsGenericParameters || element.IsByRef || element == typeof(void)) throw new Exception("Unproved owned vector element.");
                    string type = ValueCallType(element), array = ValueCallType(element.MakeArrayType());
                    string argumentsSource = string.Join(", ", methodArgs.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    string id = "Allocation" + helpers.Count;
                    helpers.Add("private delegate " + array + " " + id + "Delegate(int length);"
                        + "private static readonly System.Lazy<" + id + "Delegate> " + id + " = new System.Lazy<" + id + "Delegate>(() => {"
                        + "var module = typeof(" + ValueCallType(original.DeclaringType) + ").Module;"
                        + "var element = module.ResolveType(" + elementToken + ", " + typeContext + ", new System.Type[]{" + argumentsSource + "});"
                        + "var method = new System.Reflection.Emit.DynamicMethod(\"owned-newarr-leaf\", typeof(" + array + "), new System.Type[]{typeof(int)}, module, true);"
                        + "var il = method.GetILGenerator();il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);il.Emit(System.Reflection.Emit.OpCodes.Newarr, element);il.Emit(System.Reflection.Emit.OpCodes.Ret);"
                        + "return (" + id + "Delegate)method.CreateDelegate(typeof(" + id + "Delegate)); });"
                        + "private static " + array + " __cil_newarr_" + elementToken.ToString("X8") + "(int length) => " + id + ".Value(length);");
                }
                foreach (var operation in row.GetProperty("elements").EnumerateArray()) {
                    int elementToken = operation.GetProperty("token").GetInt32();
                    string op = operation.GetProperty("op").GetString();
                    bool store = op == "stelem", address = op == "ldelema";
                    Type element = fixture.ManifestModule.ResolveType(elementToken, typeArgs, methodArgs);
                    Type arrayType = original.GetParameters()[0].ParameterType;
                    if (!arrayType.IsSZArray || arrayType.GetElementType() != element) throw new Exception("Owned typed element differs from vector.");
                    string type = ValueCallType(element), array = ValueCallType(arrayType), id = "Element" + helpers.Count;
                    string returned = store ? "void" : (address ? "ref " : "") + type;
                    string resultType = store ? "typeof(void)" : "typeof(" + type + ")" + (address ? ".MakeByRefType()" : "");
                    string parameters = array + " values, int index" + (store ? ", " + type + " value" : "");
                    string shape = "new System.Type[]{typeof(" + array + "),typeof(int)" + (store ? ",typeof(" + type + ")" : "") + "}";
                    string argumentsSource = string.Join(", ", methodArgs.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    helpers.Add("private delegate " + returned + " " + id + "Delegate(" + parameters + ");"
                        + "private static readonly System.Lazy<" + id + "Delegate> " + id + " = new System.Lazy<" + id + "Delegate>(() => {"
                        + "var module = typeof(" + ValueCallType(original.DeclaringType) + ").Module;"
                        + "var element = module.ResolveType(" + elementToken + ", " + typeContext + ", new System.Type[]{" + argumentsSource + "});"
                        + "var method = new System.Reflection.Emit.DynamicMethod(\"owned-element-leaf\", " + resultType + ", " + shape + ", module, true);"
                        + "var il = method.GetILGenerator();il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);il.Emit(System.Reflection.Emit.OpCodes.Ldarg_1);"
                        + (store ? "il.Emit(System.Reflection.Emit.OpCodes.Ldarg_2);" : "")
                        + "il.Emit(System.Reflection.Emit.OpCodes." + (store ? "Stelem" : address ? "Ldelema" : "Ldelem") + ", element);il.Emit(System.Reflection.Emit.OpCodes.Ret);"
                        + "return (" + id + "Delegate)method.CreateDelegate(typeof(" + id + "Delegate)); });"
                        + "private static " + returned + " __cil_" + op + "_" + elementToken.ToString("X8") + "(" + parameters + ")"
                        + (store ? " { " + id + ".Value(values,index,value); }" : " => " + (address ? "ref " : "") + id + ".Value(values,index);"));
                }
                results.Add(new { token, header, aliases, helpers, carrier = "TypedArray" + number++,
                    types = typeArgs.Select(t => t.AssemblyQualifiedName).ToArray(),
                    argument = methodArgs.Length == 0 ? null : methodArgs[0].AssemblyQualifiedName });
            }
        }
        Console.WriteLine(JsonSerializer.Serialize(results));
    }

    static void TypedArrayCheck(string binary, string plan)
    {
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(binary));
        using var json = JsonDocument.Parse(System.IO.File.ReadAllText(plan));
        int comparisons = 0, selections = 0;
        foreach (var row in json.RootElement.EnumerateArray()) {
            var original = (MethodInfo)fixture.ManifestModule.ResolveMethod(row.GetProperty("token").GetInt32());
            var ownerArgs = row.GetProperty("types").EnumerateArray().Select(t => Type.GetType(t.GetString(), true)).ToArray();
            if (ownerArgs.Length != 0) original = original.DeclaringType.MakeGenericType(ownerArgs).GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
                .Single(m => m.MetadataToken == original.MetadataToken);
            if (original.IsGenericMethodDefinition) original = original.MakeGenericMethod(Type.GetType(row.GetProperty("argument").GetString(), true));
            var lowered = generated.GetType(row.GetProperty("carrier").GetString() + ".Recovered")?.GetMethod(original.Name)
                ?? throw new Exception("Recovered vector method missing.");
            if (original.ReturnType != lowered.ReturnType || !original.GetParameters().Select(p => p.ParameterType)
                .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType))) throw new Exception("Typed vector ABI differs.");
            selections++;
            if (original.Name.StartsWith("New", StringComparison.Ordinal) || original.Name == "UnsupportedArray" && original.DeclaringType.Namespace == "EnumCases") {
                if (!original.ReturnType.IsSZArray || original.GetParameters().Length != 1 || original.GetParameters()[0].ParameterType != typeof(int))
                    throw new Exception("Owned allocation ABI differs.");
                bool Zero(object value, Type type) {
                    if (!type.IsValueType || Nullable.GetUnderlyingType(type) != null) return value == null;
                    if (type == typeof(double)) return BitConverter.DoubleToInt64Bits((double)value) == 0;
                    if (type.IsPrimitive || type.IsEnum) return Equals(value, Activator.CreateInstance(type));
                    return type.GetFields(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance).All(f => Zero(f.GetValue(value), f.FieldType));
                }
                foreach (int length in new[] { -1, 0, 1, 5 }) {
                    (Array value, Type error) Allocate(MethodInfo method) {
                        try { return ((Array)ManagedInvoker(method)(new object[] { length }), null); } catch (Exception error) { return (null, error.GetType()); }
                    }
                    var before = Allocate(original); var after = Allocate(lowered);
                    if (before.error != after.error || before.error == null && (before.value == null || after.value == null
                        || before.value.GetType() != after.value.GetType() || before.value.Length != after.value.Length
                        || !after.value.Cast<object>().All(value => Zero(value, original.ReturnType.GetElementType()))))
                        throw new Exception("Recovered vector behavior differs: " + original.Name + " allocation");
                    if (before.error != (length < 0 ? typeof(OverflowException) : null)
                        || length >= 0 && (before.value.GetType() != original.ReturnType || before.value.Length != length
                        || !before.value.Cast<object>().All(value => Zero(value, original.ReturnType.GetElementType())))) throw new Exception("Owned allocation expectation differs.");
                    if (length >= 0 && (ReferenceEquals(before.value, after.value) || ReferenceEquals(after.value, Allocate(lowered).value)))
                        throw new Exception("Recovered vector behavior differs: cached allocation");
                    comparisons++;
                }
                continue;
            }
            if (original.Name == "AddressOrder" || original.Name == "StoreOrder") {
                bool acquired = original.Name == "AddressOrder";
                var trace = fixture.GetType("TypedArrayCases.Ops").GetField("Trace") ?? throw new Exception("Owned vector trace missing.");
                object nextObject = new object(), previous = new object();
                foreach (int length in new[] { -1, 0, 2 })
                foreach (int index in new[] { -1, 0, 3 })
                foreach (bool covariant in new[] { false, true })
                foreach (object next in new object[] { null, nextObject, "next" })
                foreach (bool fail in new[] { false, true }) {
                    Array input = length < 0 ? null : Array.CreateInstance(covariant ? typeof(string) : typeof(object), length);
                    if (length > 0) input.SetValue(covariant ? "before" : previous, 0);
                    Array initial = (Array)input?.Clone();
                    (object value, Type error, int stamp, object[] state) InvokeOrdered(MethodInfo method) {
                        if (input != null) Array.Copy(initial, input, input.Length);
                        trace.SetValue(null, 0);
                        var result = Capture(() => ManagedInvoker(method)(new object[] { input, index, next, fail }), new List<State>());
                        return (result.Value, result.Error, (int)trace.GetValue(null), input?.Cast<object>().ToArray());
                    }
                    var before = InvokeOrdered(original); var after = InvokeOrdered(lowered);
                    bool sameState = before.state == null ? after.state == null : before.state.Zip(after.state).All(p => ReferenceEquals(p.First, p.Second));
                    if (before.error != after.error || before.stamp != after.stamp || !ReferenceEquals(before.value, after.value) || !sameState)
                        throw new Exception("Recovered vector behavior differs: " + original.Name + " acquisition/argument order");
                    Type acquisitionError = length < 0 ? typeof(NullReferenceException)
                        : index < 0 || index >= length ? typeof(IndexOutOfRangeException) : covariant ? typeof(ArrayTypeMismatchException) : null;
                    Type expectedError = acquired && acquisitionError != null ? acquisitionError : fail ? typeof(NullReferenceException)
                        : length < 0 ? typeof(NullReferenceException) : index < 0 || index >= length ? typeof(IndexOutOfRangeException)
                        : covariant && next != null && next is not string ? typeof(ArrayTypeMismatchException) : null;
                    int expectedTrace = acquired && acquisitionError != null ? 0 : 12;
                    if (before.error != expectedError || before.stamp != expectedTrace)
                        throw new Exception("Owned ordered vector expectation differs: " + original.Name);
                    comparisons++;
                }
                continue;
            }
            Type parameter = original.GetParameters()[0].ParameterType;
            bool lengthOnly = original.Name.EndsWith("Length", StringComparison.Ordinal);
            bool store = original.Name.StartsWith("Store", StringComparison.Ordinal);
            bool address = original.Name.StartsWith("Address", StringComparison.Ordinal);
            bool exchange = original.Name.StartsWith("Exchange", StringComparison.Ordinal);
            bool writes = store || address || exchange;
            if (!parameter.IsSZArray || original.GetParameters().Length != (lengthOnly ? 1 : writes ? 3 : 2)
                || !lengthOnly && original.GetParameters()[1].ParameterType != typeof(int)
                || writes && original.GetParameters()[2].ParameterType != parameter.GetElementType())
                throw new Exception("Owned vector input shape differs.");
            object shared = new object();
            string first = new string("first".ToCharArray()), second = new string("second".ToCharArray());
            object Sample(Type type, int variant) {
                Type nullable = Nullable.GetUnderlyingType(type);
                if (nullable != null) return variant == 0 ? null : Sample(nullable, variant);
                if (type == typeof(string)) return variant == 0 ? null : variant == 1 ? first : second;
                if (type == typeof(object)) return variant == 0 ? null : variant == 1 ? shared : first;
                if (type == typeof(bool)) return __cil_bool_store(variant);
                if (type == typeof(short)) return variant == 0 ? short.MinValue : variant == 1 ? short.MaxValue : (short)-1;
                if (type == typeof(char)) return variant == 0 ? (char)0 : variant == 1 ? (char)0xd800 : (char)0xffff;
                if (type == typeof(byte)) return (byte)(variant == 0 ? 0 : variant == 1 ? 0x80 : 0xff);
                if (type == typeof(long)) return variant == 0 ? long.MinValue : variant == 1 ? long.MaxValue : -1L;
                if (type == typeof(float)) return variant == 0 ? -0.0f : variant == 1 ? BitConverter.Int32BitsToSingle(0x7fc00123) : float.MaxValue;
                if (type == typeof(double)) return variant == 0 ? -0.0 : variant == 1 ? BitConverter.Int64BitsToDouble(0x7ff8000000000123) : double.MaxValue;
                if (type == typeof(int)) return variant == 0 ? int.MinValue : variant == 1 ? 17 : -1;
                if (type == fixture.GetType("TypedArrayCases.Pair")) {
                    object value = Activator.CreateInstance(type);
                    type.GetField("Number").SetValue(value, Sample(typeof(int), variant));
                    type.GetField("Tag").SetValue(value, Sample(typeof(object), variant));
                    return value;
                }
                return Activator.CreateInstance(type);
            }
            foreach (int length in new[] { -1, 0, 1, 7 })
            foreach (int index in lengthOnly ? new[] { 0 } : new[] { -1, 0, 1, 8, int.MaxValue })
            foreach (bool covariant in parameter == typeof(object[]) ? new[] { false, true } : new[] { false })
            foreach (object next in writes ? Enumerable.Range(0,3).Select(i => Sample(parameter.GetElementType(), i)).ToArray() : new object[] { null }) {
                Type element = covariant ? typeof(string) : parameter.GetElementType();
                Array input = length < 0 ? null : Array.CreateInstance(element, length);
                if (input != null) for (int i = 0; i < length; i++) {
                    object value = Sample(element, (i + 1) % 3);
                    input.SetValue(value, i);
                }
                if (input != null && element == typeof(bool)) for (int i = 0; i < input.Length; i++) Buffer.SetByte(input, i, (byte)(i % 3));
                Array initial = (Array)input?.Clone();
                bool SameValue(object left, object right) => original.ReturnType == typeof(bool) && left != null && right != null
                    ? __cil_bool_load((bool)left) == __cil_bool_load((bool)right)
                    : original.ReturnType.IsValueType ? Same(left, right) : ReferenceEquals(left, right);
                bool SameArray(Array left, Array right) => left == null ? right == null : element == typeof(bool)
                    ? Enumerable.Range(0, left.Length).All(i => Buffer.GetByte(left, i) == Buffer.GetByte(right, i))
                    : left.Cast<object>().Zip(right.Cast<object>()).All(pair =>
                    element.IsValueType ? Same(pair.First, pair.Second) : ReferenceEquals(pair.First, pair.Second));
                (object value, string error, Array state) Invoke(MethodInfo method) {
                    if (input != null) Array.Copy(initial, input, input.Length);
                    object value = null; Exception error = null;
                    object[] values = lengthOnly ? new object[] { input } : writes ? new object[] { input, index, next } : new object[] { input, index };
                    try { value = ManagedInvoker(method)(values); } catch (Exception e) { error = e; }
                    return (value, error?.GetType().FullName, (Array)input?.Clone());
                }
                var before = Invoke(original); var after = Invoke(lowered);
                if (!SameValue(before.value, after.value) || before.error != after.error || !SameArray(before.state, after.state))
                    throw new Exception("Recovered vector behavior differs: " + original.Name + " expected=" + before + " actual=" + after);
                string expectedError = length < 0 ? typeof(NullReferenceException).FullName
                    : !lengthOnly && (index < 0 || index >= length) ? typeof(IndexOutOfRangeException).FullName
                    : address && covariant || writes && next != null && !element.IsInstanceOfType(next) ? typeof(ArrayTypeMismatchException).FullName : null;
                Array expected = (Array)initial?.Clone();
                if (writes && expectedError == null) expected.SetValue(next, index);
                if (before.error != expectedError || !SameArray(expected, before.state))
                    throw new Exception("Owned vector expectation differs.");
                if (before.error == null && !store && (lengthOnly ? !Equals(before.value, length) : !SameValue(before.value, initial.GetValue(index))))
                    throw new Exception("Owned vector result differs.");
                comparisons++;
            }
        }
        Console.WriteLine("Typed vector behavior: " + comparisons + " comparisons, " + selections
            + " closed selections; length, reference identity, covariance, state and null/bounds/type checks match.");
    }
}
