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
    static int allocationChecks;

    // Compare what the caller can observe about a fresh object: its runtime type and
    // every public instance field. This executes only owned fixtures.
    static string AllocationDescriptor(object value)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        var parts = new List<string> { type.FullName };
        foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
            parts.Add(field.Name + "=" + (field.GetValue(value) ?? "null"));
        return string.Join(",", parts);
    }

    static void ExternalAllocationMain(string[] args)
    {
        if (args.Length != 4 || fixture.GetType("ExternalAllocationCases.Ops") == null)
            throw new Exception("Expected owned external-allocation fixture and its library.");
        string libraryPath = System.IO.Path.GetFullPath(args[3]);
        Assembly resolved = null;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
        {
            if (name.Name == "ExternalAllocationLibrary") return resolved ??= context.LoadFromAssemblyPath(libraryPath);
            return null;
        };
        resolved = AssemblyLoadContext.Default.LoadFromAssemblyPath(libraryPath);
        Type ops = fixture.GetType("ExternalAllocationCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExternalAllocationCases.Recovered")
            ?? throw new Exception("Recovered external-allocation carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned external-allocation selection is empty.");

        object[] Candidates(Type type)
        {
            if (type == typeof(int)) return new object[] { 0, 7, -3 };
            if (type == typeof(string)) return new object[] { "x", "" };
            if (!type.IsValueType) return new object[] { null };
            throw new Exception("Unexpected owned allocation parameter type: " + type);
        }

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered external-allocation method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered external-allocation signature differs: " + original.Name);
            var cases = new List<object[]>();
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            else
                foreach (object first in Candidates(parameters[0]))
                {
                    if (parameters.Length == 1) cases.Add(new[] { first });
                    else foreach (object second in Candidates(parameters[1]))
                        cases.Add(new[] { first, second });
                }
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new List<State>());
                string expected = before.Error != null ? "throw:" + before.Error.Name : AllocationDescriptor(before.Value);
                string actual = after.Error != null ? "throw:" + after.Error.Name : AllocationDescriptor(after.Value);
                if (expected != actual)
                    throw new Exception("Recovered external allocation differs: " + original.Name
                        + " expected=" + expected + " actual=" + actual
                        + " args=" + string.Join("|", arguments.Select(a => a == null ? "null" : a.ToString())));
                allocationChecks++;
            }
        }
        if (allocationChecks == 0) throw new Exception("Owned external-allocation checks are empty.");
        Console.WriteLine("External allocation behavior: " + allocationChecks
            + " comparisons, " + selected.Length
            + " recovered methods; owner type, field contents, argument order, identity and exceptions match.");
    }
}
