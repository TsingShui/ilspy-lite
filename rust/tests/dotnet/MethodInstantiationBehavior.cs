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
    static void MethodInstantiationMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned MethodSpec fixture and carrier.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type Named(Assembly assembly, string name) => assembly.GetType("MethodInstantiationCases." + name)
            ?? throw new Exception("Owned method-instantiation type missing: " + name);
        Type ops = Named(fixture, "Ops");
        Type carrier = Named(generated, "RecoveredOps");
        Type payload = Named(fixture, "Payload");
        FieldInfo originalTrace = Named(fixture, "Helpers").GetField("Trace");
        FieldInfo recoveredTrace = Named(generated, "RecoveredHelpers").GetField("Trace");
        object shared = new object();
        object Make(Type type)
        {
            object value = Activator.CreateInstance(type);
            foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                if (field.FieldType == typeof(string)) field.SetValue(value, "initial");
                else if (field.FieldType == typeof(int)) field.SetValue(value, 17);
                else if (field.FieldType == typeof(object)) field.SetValue(value, shared);
                else throw new Exception("Unexpected owned field: " + field);
            return value;
        }
        string Describe(object value, object[] arguments)
        {
            if (value == null) return "null";
            if (ReferenceEquals(value, shared)) return "shared";
            if (value.GetType().IsPrimitive || value is string) return value.GetType().Name + ":" + value;
            if (!value.GetType().IsValueType)
                for (int index = 0; index < arguments.Length; index++)
                    if (ReferenceEquals(value, arguments[index])) return "arg:" + index;
            return value.GetType().Name.Replace("Recovered", "") + "{" + string.Join(",",
                value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name)
                    .Select(field => field.Name + "=" + Describe(field.GetValue(value), Array.Empty<object>()))) + "}";
        }
        object[] Candidates(Type type, bool lowered)
        {
            if (type == typeof(int)) return new object[] { 0, 2, -2 };
            if (type == typeof(string)) return new object[] { null, "", "hello" };
            if (type == typeof(object)) return new[] { null, shared, "hello" };
            if (type == payload) return new[] { Make(payload), Activator.CreateInstance(payload) };
            if (type.IsGenericType && type.GetGenericTypeDefinition().Name.EndsWith("Caller`1"))
                return new[] { null, Make(type) };
            if (type.IsGenericType && type.GetGenericTypeDefinition().Name.EndsWith("Store`1"))
            {
                Type derived = Named(lowered ? generated : fixture, lowered ? "RecoveredDerived`1" : "Derived`1")
                    .MakeGenericType(type.GenericTypeArguments);
                return new[] { null, Make(type), Make(derived) };
            }
            throw new Exception("Unexpected MethodSpec parameter: " + type);
        }
        int comparisons = 0, selections = 0;
        foreach (MethodInfo definition in carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .OrderBy(m => m.Name))
        {
            MethodInfo originalDefinition = ops.GetMethod(definition.Name)
                ?? throw new Exception("Owned counterpart missing: " + definition.Name);
            Type[][] instantiations = definition.IsGenericMethodDefinition
                ? new[] { new[] { typeof(int) }, new[] { typeof(string) }, new[] { typeof(object) }, new[] { payload } }
                : new[] { Type.EmptyTypes };
            foreach (Type[] typeArguments in instantiations)
            {
                MethodInfo original = typeArguments.Length == 0 ? originalDefinition : originalDefinition.MakeGenericMethod(typeArguments);
                MethodInfo lowered = typeArguments.Length == 0 ? definition : definition.MakeGenericMethod(typeArguments);
                Type[] beforeTypes = original.GetParameters().Select(p => p.ParameterType).ToArray();
                Type[] afterTypes = lowered.GetParameters().Select(p => p.ParameterType).ToArray();
                if (beforeTypes.Length != afterTypes.Length) throw new Exception("Recovered arity differs.");
                var beforeCases = new List<object[]> { Array.Empty<object>() };
                var afterCases = new List<object[]> { Array.Empty<object>() };
                foreach (Type type in beforeTypes)
                    beforeCases = beforeCases.SelectMany(prefix => Candidates(type, false)
                        .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
                foreach (Type type in afterTypes)
                    afterCases = afterCases.SelectMany(prefix => Candidates(type, true)
                        .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
                if (beforeCases.Count != afterCases.Count) throw new Exception("Candidate shape differs.");
                selections++;
                for (int index = 0; index < beforeCases.Count; index++)
                {
                    object[] beforeArgs = beforeCases[index];
                    object[] afterArgs = afterCases[index];
                    originalTrace.SetValue(null, 0);
                    recoveredTrace.SetValue(null, 0);
                    Outcome before = Capture(() => OriginalCall(original)(null, beforeArgs), new List<State>());
                    Outcome after = Capture(() => lowered.Invoke(null, afterArgs), new List<State>());
                    string State(object[] values) => string.Join("|", values.Select(value => Describe(value, Array.Empty<object>())));
                    if (before.Error != after.Error || Describe(before.Value, beforeArgs) != Describe(after.Value, afterArgs)
                        || State(beforeArgs) != State(afterArgs)
                        || !Equals(originalTrace.GetValue(null), recoveredTrace.GetValue(null)))
                        throw new Exception("Recovered MethodSpec differs: " + original.Name
                            + " expected=" + (before.Error?.Name ?? Describe(before.Value, beforeArgs))
                            + " actual=" + (after.Error?.Name ?? Describe(after.Value, afterArgs))
                            + " state=" + State(beforeArgs) + "/" + State(afterArgs)
                            + " trace=" + originalTrace.GetValue(null) + "/" + recoveredTrace.GetValue(null));
                    comparisons++;
                }
            }
        }
        Console.WriteLine("MethodSpec behavior: " + comparisons + " comparisons, " + selections
            + " selections; static and nonvirtual generic calls, reference/struct copies, arguments, state and exceptions match.");
    }
}
