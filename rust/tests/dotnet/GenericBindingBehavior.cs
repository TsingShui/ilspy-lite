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
    static void GenericBindingMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned generic-binding fixture and carrier.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type Named(Assembly assembly, string name) => assembly.GetType("GenericBindingCases." + name)
            ?? throw new Exception("Owned generic-binding type missing: " + name);
        Type payload = Named(fixture, "Payload");
        FieldInfo initialization = Named(fixture, "Tracker").GetField("Count");
        object shared = new object();
        string TypeName(Type type) => type.FullName?.Replace("Recovered", "") ?? type.Name;
        string Describe(object value, object self)
        {
            if (value == null) return "null";
            if (ReferenceEquals(value, self)) return "self";
            if (ReferenceEquals(value, shared)) return "shared";
            Type type = value.GetType();
            if (type.IsPrimitive || value is string) return type.FullName + ":" + value;
            return TypeName(type).Split('[')[0] + "{" + string.Join(",",
                type.GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name)
                    .Select(field => field.Name + "=" + Describe(field.GetValue(value), self))) + "}";
        }
        object Scalar(Type type, int seed)
        {
            if (type == typeof(int)) return seed;
            if (type == typeof(string)) return "v" + seed;
            if (type == typeof(object)) return shared;
            if (type == payload)
            {
                object value = Activator.CreateInstance(payload);
                payload.GetField("Number").SetValue(value, seed);
                payload.GetField("Reference").SetValue(value, shared);
                return value;
            }
            throw new Exception("Unexpected generic-binding scalar: " + type);
        }
        object Make(Type type, bool nullTarget = false)
        {
            object result = Activator.CreateInstance(type);
            foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                field.SetValue(result, field.Name == "Target"
                    ? nullTarget ? null : Make(field.FieldType) : Scalar(field.FieldType, field.Name == "Second" ? 23 : 7));
            return result;
        }
        Type[][] allArguments = {
            new[] { typeof(int) }, new[] { typeof(string) }, new[] { typeof(object) }, new[] { payload }
        };
        int comparisons = 0, selectedMethods = 0;
        foreach (string family in new[] { "Bag", "Proxy", "Pair", "Swapped" })
        {
            int arity = family == "Pair" || family == "Swapped" ? 2 : 1;
            Type originalDefinition = Named(fixture, family + "`" + arity);
            Type recoveredDefinition = Named(generated, "Recovered" + family + "`" + arity);
            foreach (Type[] first in allArguments)
            {
                Type[] typeArguments = arity == 1 ? first : new[] { first[0], first[0] == typeof(int) ? typeof(string) : typeof(int) };
                Type originalType = originalDefinition.MakeGenericType(typeArguments);
                Type recoveredType = recoveredDefinition.MakeGenericType(typeArguments);
                MethodInfo[] methods = recoveredType.GetMethods(BindingFlags.Public | BindingFlags.Instance
                    | BindingFlags.Static | BindingFlags.DeclaredOnly);
                if (methods.Length == 0) throw new Exception("No recovered generic-binding methods: " + family);
                foreach (MethodInfo lowered in methods.OrderBy(m => m.Name))
                {
                    MethodInfo original = originalType.GetMethod(lowered.Name) ?? throw new Exception("Owned counterpart missing.");
                    if (original.IsStatic != lowered.IsStatic) throw new Exception("Static shape differs.");
                    selectedMethods++;
                    ParameterInfo[] parameters = original.GetParameters();
                    if (parameters.Length > 1) throw new Exception("Unexpected owned arity.");
                    foreach (bool nullTarget in new[] { false, true })
                    foreach (int seed in new[] { 0, 11 })
                    {
                        initialization.SetValue(null, 0);
                        object beforeSelf = Make(originalType, nullTarget);
                        object[] beforeArgs = parameters.Select(p => Scalar(p.ParameterType, seed)).ToArray();
                        Outcome before = Capture(() => OriginalCall(original)(beforeSelf, beforeArgs), new List<State>());
                        int beforeInitialization = (int)initialization.GetValue(null);
                        initialization.SetValue(null, 0);
                        object afterSelf = Make(recoveredType, nullTarget);
                        object[] afterArgs = parameters.Select(p => Scalar(p.ParameterType, seed)).ToArray();
                        Outcome after = Capture(() => lowered.Invoke(afterSelf, afterArgs), new List<State>());
                        int afterInitialization = (int)initialization.GetValue(null);
                        if (before.Error != after.Error || Describe(before.Value, beforeSelf) != Describe(after.Value, afterSelf)
                            || Describe(beforeSelf, null) != Describe(afterSelf, null)
                            || beforeInitialization != afterInitialization)
                            throw new Exception("Recovered generic binding differs: " + family + "." + original.Name
                                + " expected=" + (before.Error?.Name ?? Describe(before.Value, beforeSelf))
                                + " actual=" + (after.Error?.Name ?? Describe(after.Value, afterSelf))
                                + " state=" + Describe(beforeSelf, null) + "/" + Describe(afterSelf, null)
                                + " init=" + beforeInitialization + "/" + afterInitialization);
                        comparisons++;
                    }
                }
            }
        }
        Console.WriteLine("Generic binding behavior: " + comparisons + " comparisons, " + selectedMethods
            + " closed method selections; caller scopes, swapped arguments, reference identity, struct state and type initialization match.");
    }
}
