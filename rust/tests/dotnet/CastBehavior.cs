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
    // Describe a result by identity relative to the inputs, so a cast that
    // returns the same instance is distinguished from one that copies.
    static string CastDescriptor(object value, object[] arguments)
    {
        if (value == null) return "null";
        for (int index = 0; index < arguments.Length; index++)
            if (ReferenceEquals(arguments[index], value)) return "arg" + index;
        if (value.GetType().IsValueType)
            return value.GetType().FullName + ":" + Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        return "type:" + value.GetType().Name;
    }

    static void CastMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("CastCases.Ops") == null)
            throw new Exception("Expected owned cast fixture.");
        Type ops = fixture.GetType("CastCases.Ops");
        Type @base = fixture.GetType("CastCases.Base");
        Type derived = fixture.GetType("CastCases.Derived");
        Type other = fixture.GetType("CastCases.Other");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("CastCases.Recovered")
            ?? throw new Exception("Recovered cast carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned cast selection is empty.");

        object baseValue = Activator.CreateInstance(@base);
        object derivedValue = Activator.CreateInstance(derived);
        object otherValue = Activator.CreateInstance(other);
        Type abstractChild = fixture.GetType("CastCases.AbstractChild")
            ?? throw new Exception("Owned abstract child missing.");
        Type abstractOther = fixture.GetType("CastCases.AbstractOther")
            ?? throw new Exception("Owned second abstract child missing.");
        object[] values = { null, baseValue, derivedValue, otherValue, "text", "",
            Activator.CreateInstance(abstractChild), Activator.CreateInstance(abstractOther), 123,
            Activator.CreateInstance(fixture.GetType("CastCases.GenericBase`1").MakeGenericType(typeof(int))) };
        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered cast method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered cast signature differs: " + original.Name);
            object[] Candidates(Type type) => type == typeof(object) ? values
                : type == @base ? new object[] { null, baseValue, derivedValue }
                : type == derived ? new object[] { null, derivedValue }
                : type == other ? new object[] { null, otherValue }
                : type == typeof(IComparable) ? new object[] { null, "text", 123 }
                : values;
            var cases = new System.Collections.Generic.List<object[]>();
            foreach (object first in Candidates(parameters.FirstOrDefault() ?? typeof(object)))
                cases.Add(new[] { first });
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                string expected = before.Error != null ? "throw:" + before.Error.Name : CastDescriptor(before.Value, arguments);
                string actual = after.Error != null ? "throw:" + after.Error.Name : CastDescriptor(after.Value, arguments);
                if (expected != actual)
                    throw new Exception("Recovered cast differs: " + original.Name + " expected=" + expected + " actual=" + actual);
                checks++;
            }
        }
        Console.WriteLine($"Cast behavior: {checks} comparisons, {selected.Length} recovered methods; isinst/castclass identity, null flow, InvalidCastException and results match.");
    }
}
