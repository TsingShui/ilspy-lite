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
    static void AbstractMemberMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned abstract-member fixture and carrier.");
        Type ops = fixture.GetType("AbstractMemberCases.Ops") ?? throw new Exception("Owned Ops missing.");
        Type baseType = fixture.GetType("AbstractMemberCases.Base`1").MakeGenericType(typeof(int));
        Type derived = fixture.GetType("AbstractMemberCases.Derived`1").MakeGenericType(typeof(int));
        Type other = fixture.GetType("AbstractMemberCases.Other`1").MakeGenericType(typeof(int));
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("AbstractMemberCases.Recovered") ?? throw new Exception("Recovered carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (selected.Length == 0) throw new Exception("Owned selection is empty.");
        object[] Candidates(Type type)
        {
            if (type == baseType) return new[] { null, Activator.CreateInstance(derived), Activator.CreateInstance(other) };
            if (type == typeof(int)) return new object[] { 0, 7, -2 };
            throw new Exception("Unexpected parameter: " + type);
        }
        object Clone(object value) => value == null || value.GetType().IsValueType
            ? value : Activator.CreateInstance(value.GetType());
        string States(object[] values) => string.Join("|", values.Select(value =>
            value == null ? "null" : value.GetType().FullName + ":"
                + string.Join(",", value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance)
                    .Select(field => field.Name + "=" + field.GetValue(value)))));
        string Describe(object value) => value == null ? "null" :
            value.GetType().IsPrimitive ? Convert.ToString(value) : value.GetType().FullName;

        int comparisons = 0;
        foreach (MethodInfo lowered in selected.OrderBy(m => m.Name))
        {
            MethodInfo original = ops.GetMethod(lowered.Name) ?? throw new Exception("Owned counterpart missing.");
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (original.ReturnType != lowered.ReturnType
                || !parameters.SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                throw new Exception("Recovered signature differs: " + original.Name);
            var cases = new List<object[]> { Array.Empty<object>() };
            foreach (Type parameter in parameters)
                cases = cases.SelectMany(prefix => Candidates(parameter)
                    .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
            foreach (object[] arguments in cases)
            {
                object[] beforeArgs = arguments.Select(Clone).ToArray();
                object[] afterArgs = arguments.Select(Clone).ToArray();
                Outcome before = Capture(() => OriginalCall(original)(null, beforeArgs), new List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, afterArgs), new List<State>());
                if (before.Error != after.Error || Describe(before.Value) != Describe(after.Value)
                    || States(beforeArgs) != States(afterArgs))
                    throw new Exception("Recovered abstract member differs: " + original.Name
                        + " expected=" + (before.Error?.Name ?? Describe(before.Value))
                        + " actual=" + (after.Error?.Name ?? Describe(after.Value))
                        + " state=" + States(beforeArgs) + "/" + States(afterArgs));
                comparisons++;
            }
        }
        Console.WriteLine("Abstract member behavior: " + comparisons + " comparisons, "
            + selected.Length + " methods; override dispatch, null checks and receiver state match.");
    }
}
