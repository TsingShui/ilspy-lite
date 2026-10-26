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
using System.Text.Json;

internal static partial class GenericReferenceRoundtrip
{
    static string ValueCallType(Type type)
    {
        if (type == typeof(void)) return "void";
        if (type.IsGenericParameter) return type.Name;
        if (type.IsByRef) return ValueCallType(type.GetElementType());
        if (type.IsGenericType) return "global::" + type.GetGenericTypeDefinition().FullName.Split('`')[0]
            + "<" + string.Join(", ", type.GetGenericArguments().Select(ValueCallType)) + ">";
        return TypeSource(type);
    }

    static void ValueCallPlan(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned value-call plan/library.");
        AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(args[2]));
        var written = new HashSet<string>();
        foreach (var item in plan.RootElement.EnumerateArray())
        {
            int callerToken = int.Parse(item.GetProperty("caller").GetString(), System.Globalization.NumberStyles.HexNumber);
            var caller = fixture.ManifestModule.ResolveMethod(callerToken);
            foreach (var operand in item.GetProperty("calls").EnumerateArray().Concat(item.GetProperty("constructors").EnumerateArray()))
            {
                string hex = operand.GetString();
                if (!written.Add(hex)) continue;
                int token = int.Parse(hex, System.Globalization.NumberStyles.HexNumber);
                var target = fixture.ManifestModule.ResolveMethod(token,
                    caller.DeclaringType.GetGenericArguments(), caller.IsGenericMethod ? caller.GetGenericArguments() : Type.EmptyTypes);
                bool constructor = target.IsConstructor;
                Type result = constructor ? target.DeclaringType : ((MethodInfo)target).ReturnType;
                if (!target.DeclaringType.IsValueType || !target.DeclaringType.IsGenericType || target.IsGenericMethod
                    || result.IsByRef || target.GetParameters().Any(p => p.ParameterType.IsByRef))
                    throw new Exception("Unproved owned value-call target: " + target);
                bool generic = target.ContainsGenericParameters;
                string arguments = generic ? "<T>" : "";
                var parameters = target.GetParameters();
                var declarations = parameters.Select((p, i) => ValueCallType(p.ParameterType) + " p" + i).ToList();
                var values = parameters.Select((p, i) => "p" + i).ToList();
                if (!target.IsStatic && !constructor)
                {
                    declarations.Insert(0, "ref " + ValueCallType(target.DeclaringType) + " receiver");
                    values.Insert(0, "ref receiver");
                }
                string call;
                string typeArguments = generic ? "<T>" : "<" + string.Join(", ", target.DeclaringType.GetGenericArguments().Select(ValueCallType)) + ">";
                if (constructor)
                {
                    string prefix = "private static " + ValueCallType(result) + " __cil_value_newobj_" + hex
                        + arguments + "(" + string.Join(", ", declarations) + ")";
                    if (target.DeclaringType.Assembly == fixture)
                        Console.WriteLine(prefix + " { " + ValueCallType(result) + " storage = default; CellCtor" + typeArguments
                            + "(ref storage" + (values.Count == 0 ? "" : ", " + string.Join(", ", values)) + "); return storage; }");
                    else Console.WriteLine(prefix + " => new " + ValueCallType(result) + "(" + string.Join(", ", values) + ");");
                    continue;
                }
                if (target.DeclaringType.Assembly == fixture)
                    call = "Cell" + target.Name + (generic ? "<T>" : "<" + string.Join(", ", target.DeclaringType.GetGenericArguments().Select(ValueCallType)) + ">")
                        + "(" + string.Join(", ", values) + ")";
                else
                    call = (target.IsStatic ? ValueCallType(target.DeclaringType) : "receiver") + "." + target.Name
                        + "(" + string.Join(", ", parameters.Select((p, i) => "p" + i)) + ")";
                Console.WriteLine("private static " + ValueCallType(result) + " __cil_value_call_" + hex
                    + arguments + "(" + string.Join(", ", declarations) + ")"
                    + (result == typeof(void) ? " { " + call + "; }" : " => " + call + ";"));
            }
        }
    }

    static void ValueCallMain(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned value-call carrier/library.");
        var library = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type ops = fixture.GetType("ValueCallCases.Ops") ?? throw new Exception("Owned value ops missing.");
        Type carrier = generated.GetType("ValueCallCases.Recovered") ?? throw new Exception("Recovered value ops missing.");
        object shared = new object();
        string first = new string("owned-first".ToCharArray()), second = new string("owned-second".ToCharArray());
        object Sample(Type type, int variant)
        {
            if (type == typeof(int)) return variant == 0 ? 0 : variant == 1 ? int.MinValue : 7;
            if (type == typeof(string)) return variant == 0 ? null : variant == 1 ? first : second;
            if (type == typeof(object)) return variant == 0 ? null : shared;
            Type underlying = Nullable.GetUnderlyingType(type);
            if (underlying != null) return variant == 0 ? null : Sample(underlying, variant);
            if (type.IsByRef)
            {
                Type cell = typeof(ManagedArgument<>).MakeGenericType(type.GetElementType());
                object value = Activator.CreateInstance(cell);
                cell.GetField("Value").SetValue(value, Sample(type.GetElementType(), variant));
                return value;
            }
            object result = Activator.CreateInstance(type);
            foreach (var field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                field.SetValue(result, Sample(field.FieldType, variant));
            return result;
        }
        string Describe(object value)
        {
            if (value == null) return "null";
            if (ReferenceEquals(value, shared)) return "shared";
            if (value is string) return ReferenceEquals(value, first) ? "first" : ReferenceEquals(value, second) ? "second" : "new-string";
            if (value.GetType().IsPrimitive) return value.GetType().FullName + ":" + value;
            return value.GetType().FullName + "{" + string.Join(",", value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance)
                .OrderBy(f => f.Name).Select(f => f.Name + "=" + Describe(f.GetValue(value)))) + "}";
        }
        var probes = new[] { fixture.GetType("ValueCallCases.Probe"), library.GetType("ValueCallLibrary.Probe") };
        if (probes.Any(p => p == null)) throw new Exception("Owned constructor probe missing.");
        var fields = probes.SelectMany(p => p.GetFields(BindingFlags.Public | BindingFlags.Static)).OrderBy(f => f.DeclaringType.FullName + "." + f.Name).ToArray();
        void Reset(int divisor)
        {
            foreach (var field in fields) field.SetValue(null, field.Name == "Divisor" ? divisor : field.Name == "Trace" ? 3 : 7);
        }
        string State() => string.Join("|", fields.Select(f => f.DeclaringType.FullName + "." + f.Name + "=" + f.GetValue(null)));
        int checks = 0, closures = 0;
        var definitions = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Where(m => !m.Name.StartsWith("Cell", StringComparison.Ordinal) && m.Name != "ProbeArgument").ToArray();
        if (definitions.Length == 0) throw new Exception("Owned value call selection empty.");
        foreach (var definition in definitions)
        {
            var originalDefinition = ops.GetMethod(definition.Name) ?? throw new Exception("Original value call missing.");
            foreach (var types in definition.IsGenericMethodDefinition
                ? new[] { new[] { typeof(int) }, new[] { typeof(string) }, new[] { typeof(object) },
                    new[] { typeof(int?) }, new[] { fixture.GetType("ValueCallCases.Cell`1").MakeGenericType(typeof(string)) } } : new[] { Type.EmptyTypes })
            {
                var original = types.Length == 0 ? originalDefinition : originalDefinition.MakeGenericMethod(types);
                var lowered = types.Length == 0 ? definition : definition.MakeGenericMethod(types);
                var parameters = original.GetParameters();
                if (original.ReturnType != lowered.ReturnType || !parameters.Select(p => p.ParameterType)
                    .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType))) throw new Exception("Value call ABI differs.");
                closures++;
                foreach (int initial in new[] { 0, 1, 2 })
                foreach (int next in new[] { 0, 1, 2 })
                {
                    object Input(ParameterInfo parameter, int index)
                    {
                        if (parameter.Name == "divisor") return next == 0 ? 0 : next == 1 ? 2 : -2;
                        if (definition.Name.StartsWith("Unbox", StringComparison.Ordinal))
                            return initial == 0 ? null : initial == 1 ? Sample(original.ReturnType, next) : shared;
                        return Sample(parameter.ParameterType, index == 0 ? initial : next);
                    }
                    object[] beforeArgs = parameters.Select(Input).ToArray();
                    object[] afterArgs = parameters.Select(Input).ToArray();
                    Reset(next == 0 ? 0 : 2);
                    Outcome before = Capture(() => ManagedInvoker(original)(beforeArgs), new List<State>());
                    string expected = Describe(before.Value), beforeState = string.Join("|", beforeArgs.Select(Describe)), staticState = State();
                    Reset(next == 0 ? 0 : 2);
                    Outcome after = Capture(() => ManagedInvoker(lowered)(afterArgs), new List<State>());
                    if (before.Error != after.Error || expected != Describe(after.Value) || beforeState != string.Join("|", afterArgs.Select(Describe)) || staticState != State())
                        throw new Exception("Recovered value call differs: " + definition.Name + " expected=" + expected + "/" + beforeState
                            + " actual=" + Describe(after.Value) + "/" + string.Join("|", afterArgs.Select(Describe)));
                    if (definition.Name == "Box" || definition.Name == "CopyBox" || definition.Name == "BoxGeneric" && types[0].IsValueType)
                    {
                        if (before.Value != null && ReferenceEquals(before.Value, after.Value))
                            throw new Exception("Recovered value box reused identity.");
                        if (definition.Name == "BoxGeneric" && after.Value != null && ReferenceEquals(after.Value, afterArgs[0]))
                            throw new Exception("Recovered generic value box aliases argument box.");
                    }
                    checks++;
                }
            }
        }
        Console.WriteLine("Value call behavior: " + checks + " comparisons, " + closures
            + " closed selections; managed receiver writes, value copies, generic/reference identity and exception writes match.");
    }
}
