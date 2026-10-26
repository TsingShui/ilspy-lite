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
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    static string Identity(object value) => value == null ? "null" : "#" + System.Runtime.CompilerServices.RuntimeHelpers.GetHashCode(value);

    static void EqualityMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("EqualityCases.Ops") == null)
            throw new Exception("Expected owned equality fixture.");
        Type ops = fixture.GetType("EqualityCases.Ops");
        Type item = fixture.GetType("EqualityCases.Item");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("EqualityCases.Recovered")
            ?? throw new Exception("Recovered equality carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned equality selection is empty.");

        object first = Activator.CreateInstance(item);
        object second = Activator.CreateInstance(item);
        // Two value-equal but distinct strings separate identity from value equality.
        object[] values = { null, first, second, "xxx", new string('x', 3), first };
        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered equality method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered equality signature differs: " + original.Name);
            object[] Candidates(Type type) => type == typeof(int) ? new object[] { 0, -1 }
                : type == typeof(string) ? new object[] { null, "text" }
                : type == typeof(object) ? values : new object[] { null, first, second };
            var cases = new System.Collections.Generic.List<object[]>();
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            else if (parameters.Length == 1) foreach (object a in Candidates(parameters[0])) cases.Add(new[] { a });
            else foreach (object a in Candidates(parameters[0])) foreach (object b in Candidates(parameters[1])) cases.Add(new[] { a, b });
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                bool boxedBoolean = original.Name == "UnsupportedExternal";
                if (boxedBoolean && before.Error == null)
                {
                    if (before.Value is not bool || after.Value is not bool
                        || ReferenceEquals(before.Value, after.Value)
                        || arguments.Any(argument => ReferenceEquals(argument, after.Value))
                        || ReferenceEquals(after.Value, lowered.Invoke(null, arguments)))
                        throw new Exception("Recovered equality boxed result lost its type or fresh identity.");
                }
                string expected = before.Error != null ? "throw:" + before.Error.Name
                    : boxedBoolean ? "boxed:" + before.Value.GetType().FullName + ":" + before.Value
                    : parameters.Length == 0 || original.ReturnType != typeof(object) ? "value:" + before.Value
                    : "identity:" + Identity(before.Value);
                string actual = after.Error != null ? "throw:" + after.Error.Name
                    : boxedBoolean ? "boxed:" + after.Value.GetType().FullName + ":" + after.Value
                    : parameters.Length == 0 || original.ReturnType != typeof(object) ? "value:" + after.Value
                    : "identity:" + Identity(after.Value);
                if (expected != actual)
                    throw new Exception("Recovered equality differs: " + original.Name + " expected=" + expected + " actual=" + actual);
                checks++;
            }
        }
        Console.WriteLine($"Equality behavior: {checks} comparisons, {selected.Length} recovered methods; identity semantics, null handling, chosen references and exceptions match.");
    }
}
