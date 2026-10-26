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
    static void ExternalPropertyMain(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned property fixture and dependency.");
        Assembly library = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        AssemblyLoadContext.Default.Resolving += (context, name) =>
            name.Name == "ExternalPropertyLibrary" ? library : null;
        Type ops = fixture.GetType("ExternalPropertyCases.Ops") ?? throw new Exception("Owned property Ops missing.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExternalPropertyCases.Recovered") ?? throw new Exception("Recovered property carrier missing.");
        Type cellType = library.GetType("ExternalProperties.Cell") ?? throw new Exception("Cell missing.");
        Type[] receivers = new[] { cellType, library.GetType("ExternalProperties.Derived"), library.GetType("ExternalProperties.Hidden") };
        if (receivers.Any(t => t == null)) throw new Exception("Derived property receivers missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (selected.Length == 0) throw new Exception("Owned property selection is empty.");

        IEnumerable<FieldInfo> Fields(Type type)
        {
            while (type != null)
            {
                foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.NonPublic
                    | BindingFlags.Instance | BindingFlags.DeclaredOnly).OrderBy(f => f.Name))
                    yield return field;
                type = type.BaseType;
            }
        }
        object Clone(object value)
        {
            if (value == null || value.GetType().IsValueType || value is string) return value;
            object clone = Activator.CreateInstance(value.GetType());
            foreach (FieldInfo field in Fields(value.GetType())) field.SetValue(clone, field.GetValue(value));
            return clone;
        }
        string StateOf(object[] values) => string.Join("|", values.Select(value =>
            value == null ? "null" : value.GetType().IsValueType ? value.ToString() :
            value.GetType().FullName + ":" + string.Join(",", Fields(value.GetType()).Select(field =>
                field.DeclaringType.FullName + "." + field.Name + "=" + field.GetValue(value)))));

        object[] Candidates(Type type)
        {
            if (type == typeof(int)) return new object[] { 0, 3, -7 };
            if (type == typeof(bool)) return new object[] { false, true };
            if (type != cellType) throw new Exception("Unexpected property argument type: " + type);
            var values = new List<object> { null };
            foreach (Type receiver in receivers)
                foreach (int initial in new[] { 0, 7 })
                {
                    object value = Activator.CreateInstance(receiver);
                    cellType.GetField("Value").SetValue(value, initial);
                    values.Add(value);
                }
            return values.ToArray();
        }

        int count = 0;
        foreach (MethodInfo lowered in selected.OrderBy(m => m.Name))
        {
            MethodInfo method = ops.GetMethod(lowered.Name) ?? throw new Exception("Owned counterpart missing.");
            Type[] parameters = method.GetParameters().Select(p => p.ParameterType).ToArray();
            if (method.ReturnType != lowered.ReturnType
                || !parameters.SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                throw new Exception("Property signature differs: " + method.Name);
            var cases = new List<object[]> { Array.Empty<object>() };
            foreach (Type parameter in parameters)
                cases = cases.SelectMany(prefix => Candidates(parameter)
                    .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
            foreach (object[] arguments in cases)
            {
                object[] expectedArgs = arguments.Select(Clone).ToArray();
                object[] actualArgs = arguments.Select(Clone).ToArray();
                Outcome expected = Capture(() => OriginalCall(method)(null, expectedArgs), new List<State>());
                Outcome actual = Capture(() => lowered.Invoke(null, actualArgs), new List<State>());
                string expectedState = StateOf(expectedArgs), actualState = StateOf(actualArgs);
                if (expected.Error != actual.Error || !Equals(expected.Value, actual.Value)
                    || expectedState != actualState)
                    throw new Exception("Recovered property differs: " + method.Name
                        + " expected=" + (expected.Error?.Name ?? Convert.ToString(expected.Value))
                        + " actual=" + (actual.Error?.Name ?? Convert.ToString(actual.Value))
                        + " state=" + expectedState + "/" + actualState);
                count++;
            }
        }
        Console.WriteLine("External property behavior: " + count + " comparisons, " + selected.Length
            + " recovered methods; dispatch, null/argument exception order and receiver state match.");
    }
}
