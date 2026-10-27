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

internal static partial class GenericReferenceRoundtrip
{
    static void ExceptionMain(string[] args)
    {
        if (args.Length != 3 && args.Length != 4) throw new Exception("Expected owned fixture, recovered carrier, and optional library.");
        if (args.Length == 4)
            AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        Type ops = fixture.GetType("ExceptionRegionCases.Ops")
            ?? throw new Exception("Owned exception-region fixture missing.");
        FieldInfo trace = ops.GetField("Trace") ?? throw new Exception("Trace field missing.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExceptionRegionCases.Recovered")
            ?? throw new Exception("Recovered exception carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (selected.Length == 0) throw new Exception("Owned exception selection is empty.");
        int comparisons = 0;
        foreach (MethodInfo lowered in selected.OrderBy(m => m.Name))
        {
            MethodInfo original = ops.GetMethod(lowered.Name)
                ?? throw new Exception("Owned counterpart missing: " + lowered.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered exception signature differs: " + original.Name);
            var cases = new List<object[]>();
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            else if (parameters.SequenceEqual(new[] { typeof(int) }))
                foreach (int value in new[] { 7, 0, -2, -1, int.MinValue, int.MaxValue })
                    cases.Add(new object[] { value });
            else if (parameters.SequenceEqual(new[] { typeof(ArgumentOutOfRangeException) }))
            {
                cases.Add(new object[] { new ArgumentOutOfRangeException("original", "message") });
                cases.Add(new object[] { null });
            }
            else if (parameters.Length == 1 && typeof(Exception).IsAssignableFrom(parameters[0]))
            {
                Assembly library = AppDomain.CurrentDomain.GetAssemblies()
                    .Single(a => a.GetType("RelationLibrary.ForeignException") != null);
                Type parent = library.GetType("RelationLibrary.ForeignException");
                Type child = library.GetType("RelationLibrary.ForeignDerivedException");
                cases.Add(new object[] { null });
                cases.Add(new object[] { Activator.CreateInstance(parent) });
                cases.Add(new object[] { Activator.CreateInstance(child) });
                if (parameters[0] == typeof(Exception))
                    cases.Add(new object[] { new InvalidOperationException("unrelated") });
            }
            else throw new Exception("Unexpected owned exception parameters: " + original.Name);
            foreach (object[] arguments in cases)
            {
                trace.SetValue(null, 0);
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new List<State>());
                int expectedTrace = (int)trace.GetValue(null);
                trace.SetValue(null, 0);
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new List<State>());
                int actualTrace = (int)trace.GetValue(null);
                bool valuesMatch = original.ReturnType == typeof(object)
                    ? ReferenceEquals(before.Value, after.Value)
                    : Equals(before.Value, after.Value);
                if (original.Name == "ForeignThrow" && arguments[0] is Exception input)
                {
                    Exception originalThrown = null, loweredThrown = null;
                    try { OriginalCall(original)(null, arguments); }
                    catch (Exception error) { originalThrown = error; }
                    try { lowered.Invoke(null, arguments); }
                    catch (TargetInvocationException error) { loweredThrown = error.InnerException; }
                    if (!ReferenceEquals(originalThrown, input) || !ReferenceEquals(loweredThrown, input))
                        throw new Exception("Recovered exception behavior differs: " + original.Name + " identity");
                }
                if (before.Error != after.Error || !valuesMatch || expectedTrace != actualTrace)
                    throw new Exception("Recovered exception behavior differs: " + original.Name
                        + " expected=" + (before.Error?.Name ?? Convert.ToString(before.Value))
                        + " actual=" + (after.Error?.Name ?? Convert.ToString(after.Value))
                        + " trace=" + expectedTrace + "/" + actualTrace);
                comparisons++;
            }
        }
        if (comparisons == 0) throw new Exception("Owned exception checks are empty.");
        Console.WriteLine("Exception behavior: " + comparisons + " comparisons, " + selected.Length
            + " recovered methods; results, traces, exception types and rethrow identity match.");
    }
}
