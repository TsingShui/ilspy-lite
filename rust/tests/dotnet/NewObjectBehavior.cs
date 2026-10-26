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
    // Observable state of an allocated object, without comparing identities that
    // must legitimately differ between the original and the recovered run.
    static string Fingerprint(object value, object[] arguments)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        if (type.IsPrimitive || type == typeof(string)) return type.Name + ":" + value;
        if (type.Name != "Node") return type.Name;
        FieldInfo number = type.GetField("Value");
        FieldInfo tag = type.GetField("Tag");
        FieldInfo next = type.GetField("Next");
        object tagValue = ReadState(tag, value);
        object nextValue = ReadState(next, value);
        string tagText = tagValue == null ? "null"
            : arguments.Any(a => ReferenceEquals(a, tagValue)) ? "argument" : tagValue.GetType().Name;
        string nextText = nextValue == null ? "null"
            : ReferenceEquals(nextValue, value) ? "self" : "other";
        return "Node(" + ReadState(number, value) + "," + tagText + "," + nextText + ")";
    }

    static void NewObjectMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("NewObjectCases.Ops") == null)
            throw new Exception("Expected owned allocation fixture.");
        Type ops = fixture.GetType("NewObjectCases.Ops");
        Type node = fixture.GetType("NewObjectCases.Node");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("NewObjectCases.Recovered")
            ?? throw new Exception("Recovered allocation carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .SelectMany(method => method.IsGenericMethodDefinition
                ? new[] { typeof(int), typeof(object), node }.Select(type => method.MakeGenericMethod(type))
                : new[] { method })
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned allocation selection is empty.");

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered allocation: " + original.Name);
            if (original.IsGenericMethod) lowered = lowered.MakeGenericMethod(original.GetGenericArguments());
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered allocation signature differs: " + original.Name);
            object existing = Activator.CreateInstance(node);
            node.GetField("Value").SetValue(existing, 41);
            // Arguments follow the declared parameter types, so no case can throw
            // for the wrong reason on either side.
            object[] Build(int seed) => parameters
                .Select((p, i) => p == typeof(int) ? (object)(seed + i) : existing).ToArray();
            var tuples = new System.Collections.Generic.List<object[]>
            {
                Build(0), Build(-7), Build(int.MaxValue - 2)
            };
            if (original.Name == "UnsupportedGeneric" && !parameters[0].IsValueType) tuples.Add(new object[] { null });
            foreach (object[] arguments in tuples)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                string expected = before.Error != null ? "throw:" + before.Error.Name : Fingerprint(before.Value, arguments);
                string actual = after.Error != null ? "throw:" + after.Error.Name : Fingerprint(after.Value, arguments);
                if (expected != actual)
                    throw new Exception("Recovered allocation differs: " + original.Name + " expected=" + expected + " actual=" + actual);
                checks++;
            }
            if (original.IsGenericMethod && original.GetParameters().Length == 0 && !original.ReturnType.IsValueType)
            {
                object first = lowered.Invoke(null, null), second = lowered.Invoke(null, null);
                if (first == null || second == null || ReferenceEquals(first, second))
                    throw new Exception("Generic factory did not return fresh identities.");
                checks++;
            }
        }
        // Two separate allocations of equal content must stay distinct identities,
        // and repeated calls must each yield a fresh instance.
        MethodInfo a = carrier.GetMethod("A"), b = carrier.GetMethod("B");
        if (a != null && b != null)
        {
            object first = a.Invoke(null, null), second = b.Invoke(null, null), again = a.Invoke(null, null);
            if (second == null || again == null) throw new Exception("Allocation produced no instance.");
            if (ReferenceEquals(first, second)) throw new Exception("Distinct allocations share one identity.");
            if (ReferenceEquals(first, again)) throw new Exception("Repeated allocation reused one identity.");
            checks += 2;
        }
        Console.WriteLine($"Allocation behavior: {checks} comparisons, {selected.Length} recovered methods; fresh identities, constructor order, field state and exceptions match.");
    }
}
