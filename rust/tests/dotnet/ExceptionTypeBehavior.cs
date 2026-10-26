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
    static void ExceptionTypePlan(string path)
    {
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(path));
        foreach (var item in plan.RootElement.EnumerateArray())
        {
            int token = item.GetInt32();
            var target = (ConstructorInfo)fixture.ManifestModule.ResolveMethod(token);
            if (!typeof(Exception).IsAssignableFrom(target.DeclaringType)) throw new Exception("Owned exception constructor ancestry missing.");
            var parameters = target.GetParameters();
            Console.WriteLine("private static " + ValueCallType(target.DeclaringType) + " __cil_newobj_" + token.ToString("X8") + "("
                + string.Join(", ", parameters.Select((p, i) => ConstrainedParameter(p.ParameterType, "p" + i))) + ") => new "
                + ValueCallType(target.DeclaringType) + "(" + string.Join(", ", parameters.Select((p, i) => "p" + i)) + ");");
        }
    }

    static void ExceptionTypeLegacyCheck(string path)
    {
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(path));
        var original = fixture.GetType("ExceptionCases.Cells") ?? throw new Exception("Legacy exception fixture missing.");
        var carrier = generated.GetType("ExceptionTypeCases.Recovered") ?? throw new Exception("Legacy exception carrier missing.");
        int count = 0;
        foreach (string name in new[] { "UnsupportedType", "UnsupportedOverload" })
        {
            var beforeMethod = original.GetMethod(name) ?? throw new Exception("Legacy exception method missing.");
            var afterMethod = carrier.GetMethod(name) ?? throw new Exception("Promoted legacy exception method missing.");
            foreach (string argument in new[] { null, "", "owned-message" })
            {
                var before = (Exception)ManagedInvoker(beforeMethod)(new object[] { argument });
                var after = (Exception)ManagedInvoker(afterMethod)(new object[] { argument });
                if (before.GetType() != after.GetType() || before.Message != after.Message
                    || (before as ArgumentException)?.ParamName != (after as ArgumentException)?.ParamName
                    || ReferenceEquals(before, after) || ReferenceEquals(after, ManagedInvoker(afterMethod)(new object[] { argument })))
                    throw new Exception("Recovered legacy exception differs: " + name);
                count++;
            }
        }
        Console.WriteLine("Legacy exception promotions: " + count + " comparisons, 2 methods; overloads, messages, parameter names and freshness match.");
    }

    static void ExceptionTypeCheck(string path)
    {
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(path));
        Type ops = fixture.GetType("ExceptionTypeCases.Ops") ?? throw new Exception("Owned exception ops missing.");
        Type carrier = generated.GetType("ExceptionTypeCases.Recovered") ?? throw new Exception("Recovered exception carrier missing.");
        string first = new string("first message".ToCharArray()), second = new string("second message".ToCharArray());
        object shared = new object();
        Exception inner = new ArgumentException("inner");
        Exception existing = new InvalidOperationException(first, inner);
        Exception invalidData = new System.IO.InvalidDataException(first, inner);
        var parents = new (Type child, Type parent)[] {
            (typeof(SystemException), typeof(Exception)), (typeof(ApplicationException), typeof(Exception)),
            (typeof(ArithmeticException), typeof(SystemException)), (typeof(ArgumentException), typeof(SystemException)),
            (typeof(NullReferenceException), typeof(SystemException)), (typeof(InvalidOperationException), typeof(SystemException)),
            (typeof(IndexOutOfRangeException), typeof(SystemException)), (typeof(FormatException), typeof(SystemException)),
            (typeof(NotImplementedException), typeof(SystemException)), (typeof(NotSupportedException), typeof(SystemException)),
            (typeof(InvalidCastException), typeof(SystemException)), (typeof(System.IO.IOException), typeof(SystemException)),
            (typeof(System.IO.InvalidDataException), typeof(SystemException)), (typeof(ObjectDisposedException), typeof(InvalidOperationException)),
            (typeof(DivideByZeroException), typeof(ArithmeticException)), (typeof(OverflowException), typeof(ArithmeticException)),
            (typeof(ArgumentNullException), typeof(ArgumentException)), (typeof(ArgumentOutOfRangeException), typeof(ArgumentException)),
            (typeof(System.IO.FileNotFoundException), typeof(System.IO.IOException)),
            (typeof(System.IO.DirectoryNotFoundException), typeof(System.IO.IOException)),
            (typeof(System.IO.EndOfStreamException), typeof(System.IO.IOException))
        };
        foreach (var pair in parents)
            if (pair.child.BaseType != pair.parent) throw new Exception("Unexpected target-runtime exception ancestry: " + pair.child);
        Type custom = fixture.GetType("ExceptionTypeCases.Custom");
        Type derived = fixture.GetType("ExceptionTypeCases.Derived");
        if (custom == null || derived == null) throw new Exception("Owned exception hierarchy missing.");
        object customValue = Activator.CreateInstance(custom, first, inner);
        object derivedValue = Activator.CreateInstance(derived, second, inner);
        var methods = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (methods.Length == 0) throw new Exception("Empty exception selection.");
        int comparisons = 0;
        foreach (var lowered in methods)
        {
            var original = ops.GetMethod(lowered.Name) ?? throw new Exception("Original exception method missing.");
            var parameters = original.GetParameters();
            if (original.ReturnType != lowered.ReturnType || !parameters.Select(p => p.ParameterType)
                .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType))) throw new Exception("Exception ABI differs.");
            var cases = new List<object[]> { Array.Empty<object>() };
            foreach (var parameter in parameters)
            {
                object[] candidates = parameter.ParameterType == typeof(int) ? new object[] { 0, 1, 2, 3 }
                    : parameter.ParameterType == typeof(string) ? new object[] { null, first, second }
                    : parameter.ParameterType == typeof(object) ? new object[] { null, shared, first }
                    : parameter.ParameterType == typeof(Exception) ? new object[] { null, new Exception(first), existing }
                    : parameter.ParameterType == typeof(InvalidOperationException) ? new object[] { null, existing }
                    : parameter.ParameterType == custom ? new object[] { null, customValue, derivedValue }
                    : parameter.ParameterType == typeof(System.IO.InvalidDataException) ? new object[] { null, invalidData }
                    : throw new Exception("Owned exception sample missing: " + parameter.ParameterType);
                cases = cases.SelectMany(prefix => candidates.Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
            }
            foreach (object[] arguments in cases)
            {
                (object result, Exception error, int trace) Invoke(MethodInfo method, Type owner)
                {
                    ops.GetField("Trace").SetValue(null, 0);
                    object result = null; Exception error = null;
                    try { result = ManagedInvoker(method)(arguments); }
                    catch (Exception thrown) { error = thrown; }
                    return (result, error, (int)ops.GetField("Trace").GetValue(null));
                }
                string Identity(object value)
                {
                    if (value == null) return "null";
                    if (ReferenceEquals(value, shared)) return "shared";
                    if (ReferenceEquals(value, first)) return "first";
                    if (ReferenceEquals(value, second)) return "second";
                    if (ReferenceEquals(value, inner)) return "inner";
                    for (int i = 0; i < arguments.Length; i++) if (ReferenceEquals(value, arguments[i])) return "arg" + i;
                    return "new:" + value.GetType().FullName;
                }
                string Describe(object value)
                {
                    if (value is Exception ex) return ex.GetType().FullName + "|" + ex.Message + "|message:" + Identity(ex.Message)
                        + "|inner:" + Identity(ex.InnerException) + "|parameter:" + (ex is ArgumentException arg ? arg.ParamName : "")
                        + "|actual:" + (ex is ArgumentOutOfRangeException range ? Identity(range.ActualValue) : "")
                        + "|identity:" + Identity(ex);
                    return Identity(value);
                }
                var before = Invoke(original, ops);
                var after = Invoke(lowered, carrier);
                if (Describe(before.result) != Describe(after.result) || Describe(before.error) != Describe(after.error) || before.trace != after.trace)
                    throw new Exception("Recovered exception behavior differs: " + original.Name + " expected=" + Describe(before.result) + "/" + Describe(before.error)
                        + "/" + before.trace + " actual=" + Describe(after.result) + "/" + Describe(after.error) + "/" + after.trace);
                if (original.Name == "Construct" || original.Name == "Range" || original.Name == "ConstructLocal" || original.Name == "Io" || original.Name == "Format" || original.Name == "Order")
                {
                    if (before.error == null && ReferenceEquals(before.result, after.result)) throw new Exception("Exception construction reused identity.");
                    var again = Invoke(lowered, carrier);
                    if (after.error == null && ReferenceEquals(after.result, again.result)) throw new Exception("Exception construction cached identity.");
                }
                comparisons++;
            }
        }
        Console.WriteLine("Exception type behavior: " + comparisons + " comparisons, " + methods.Length
            + " recovered methods; exact exception identity, null throw, constructor overloads, message/inner/actual references and state match.");
    }
}
