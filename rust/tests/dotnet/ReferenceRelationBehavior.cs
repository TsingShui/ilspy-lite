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
using System.Runtime.CompilerServices;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    static void ReferenceRelationMain(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned relation fixture/carrier/library.");
        var library = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("RelationCases.Recovered") ?? throw new Exception("Owned relation carrier missing.");
        Type ops = fixture.GetType("RelationCases.Ops") ?? throw new Exception("Owned relation operations missing.");
        object shared = new object();
        object Sample(Type type, int variant)
        {
            if (type == typeof(int)) return variant == 0 ? 0 : variant == 1 ? int.MinValue : 7;
            if (type == typeof(string)) return variant == 0 ? null : new string((variant == 1 ? "first" : "second").ToCharArray());
            if (type == typeof(object)) return variant == 0 ? null : shared;
            object result = Activator.CreateInstance(type);
            foreach (var field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                field.SetValue(result, Sample(field.FieldType, variant));
            return result;
        }
        object[] Candidates(Type parameter)
        {
            if (parameter == typeof(int))
                return new object[] { -2, 0, 1, 4, 7 };
            if (parameter == typeof(object))
            {
                Type cell = fixture.GetType("RelationCases.Cell`1").MakeGenericType(typeof(string));
                return new[] { null, shared, Sample(cell, 1) };
            }
            var choices = fixture.GetTypes().Concat(library.GetTypes()).Where(t => t.IsClass && !t.IsAbstract)
                .SelectMany(type => !type.ContainsGenericParameters ? new[] { type }
                    : type.GetGenericArguments().Length == 1
                        ? new[] { type.MakeGenericType(parameter.IsGenericType ? parameter.GetGenericArguments()[0] : typeof(string)) }
                        : new[] { type.MakeGenericType(typeof(int), typeof(string)), type.MakeGenericType(typeof(string), typeof(int)) })
                .Where(parameter.IsAssignableFrom).Distinct().ToArray();
            if (choices.Length == 0) throw new Exception("Owned relation candidate missing: " + parameter);
            return new object[] { null }.Concat(choices.SelectMany(type => new[] { Sample(type, 1), Sample(type, 2) })).ToArray();
        }
        string Describe(object value, HashSet<object> seen)
        {
            if (value == null) return "null";
            Type type = value.GetType();
            if (type.IsPrimitive) return type.FullName + ":" + value;
            string identity = type.FullName + "#" + RuntimeHelpers.GetHashCode(value);
            if (!seen.Add(value)) return identity;
            return identity + "{" + string.Join(",", type.GetFields(BindingFlags.Public | BindingFlags.Instance)
                .OrderBy(f => f.Name).Select(f => f.Name + "=" + Describe(f.GetValue(value), seen))) + "}";
        }
        string Snapshot(IEnumerable<object> roots) => string.Join("|", roots.Select(value => Describe(value, new HashSet<object>(ReferenceEqualityComparer.Instance))));
        int checks = 0, closures = 0;
        var definitions = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (definitions.Length == 0) throw new Exception("Empty owned relation selection.");
        foreach (MethodInfo definition in definitions)
        {
            int arity = definition.GetGenericArguments().Length;
            Type[][] instances = arity == 0 ? new[] { Type.EmptyTypes }
                : arity == 2 ? new[] { new[] { typeof(int), typeof(string) }, new[] { typeof(string), typeof(int) } }
                : definition.Name == "CovariantGeneric" ? new[] { new[] { typeof(string) }, new[] { typeof(object) } }
                : new[] { new[] { typeof(int) }, new[] { typeof(string) }, new[] { typeof(object) } };
            foreach (Type[] types in instances)
            {
                MethodInfo lowered = arity == 0 ? definition : definition.MakeGenericMethod(types);
                MethodInfo original = ops.GetMethods().Where(m => m.Name == definition.Name && m.GetGenericArguments().Length == arity)
                        .Select(m => arity == 0 ? m : m.MakeGenericMethod(types))
                        .Single(m => m.GetParameters().Select(p => p.ParameterType)
                            .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)));
                if (original.ReturnType != lowered.ReturnType) throw new Exception("Owned relation return type differs.");
                var cases = new List<object[]> { Array.Empty<object>() };
                foreach (var parameter in lowered.GetParameters())
                    cases = cases.SelectMany(prefix => Candidates(parameter.ParameterType)
                        .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
                closures++;
                foreach (object[] values in cases)
                {
                    string initial = Snapshot(values);
                    Outcome before = Capture(() => OriginalCall(original)(null, values), new List<State>());
                    string expected = Snapshot(new[] { before.Value });
                    Outcome after = Capture(() => OriginalCall(lowered)(null, values), new List<State>());
                    if (before.Error != after.Error || expected != Snapshot(new[] { after.Value }) || initial != Snapshot(values))
                        throw new Exception("Recovered reference relation differs: " + definition.Name + "<" + string.Join(",", types.Select(t => t.Name)) + ">"
                            + " before=" + (before.Error?.Name ?? expected) + " after=" + (after.Error?.Name ?? Snapshot(new[] { after.Value }))
                            + " inputs=" + initial + " final=" + Snapshot(values));
                    checks++;
                }
            }
        }
        foreach (Type argument in new[] { typeof(int), typeof(string), typeof(object) })
        {
            Type originalType = fixture.GetType("RelationCases.Cell`1").MakeGenericType(argument);
            Type loweredType = generated.GetType("RelationCases.Recovered+RecoveredCell`1").MakeGenericType(argument);
            MethodInfo original = originalType.GetMethod("ReadCast"), lowered = loweredType.GetMethod("ReadCast");
            foreach (object input in new[] { null, shared, Sample(originalType, 1), Sample(originalType, 2) })
            foreach (bool nullReceiver in new[] { false, true })
            {
                object beforeReceiver = nullReceiver ? null : Sample(originalType, 1);
                object afterReceiver = nullReceiver ? null : Activator.CreateInstance(loweredType);
                string initial = Snapshot(new[] { input });
                Outcome before = Capture(() => OriginalCall(original)(beforeReceiver, new[] { input }), new List<State>());
                string expected = Snapshot(new[] { before.Value });
                Outcome after = Capture(() => OriginalCall(lowered)(afterReceiver, new[] { input }), new List<State>());
                if (before.Error != after.Error || expected != Snapshot(new[] { after.Value }) || initial != Snapshot(new[] { input }))
                    throw new Exception("Recovered reference relation differs: ReadCast<" + argument.Name + ">");
                checks++;
            }
            closures++;
        }
        Console.WriteLine("Reference relation behavior: " + checks + " comparisons, " + closures
            + " closed selections; reference identity, generic positions, overload selection, null/cast flow and state match.");
    }
}
