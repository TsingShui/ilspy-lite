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
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
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

    static void ValueCallFixture(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("ValueCallCases.Plain") == null) throw new Exception("Expected owned value construction fixture.");
        byte[] image = System.IO.File.ReadAllBytes(args[0]);
        using var pe = new PEReader(new System.IO.MemoryStream(image));
        var metadata = pe.GetMetadataReader(MetadataReaderOptions.None);
        var method = fixture.GetType("ValueCallCases.Ops").GetMethod("ReinitializePlain")
            ?? throw new Exception("Owned reinitialization method missing.");
        var definition = metadata.GetMethodDefinition(MetadataTokens.MethodDefinitionHandle(method.MetadataToken & 0xffffff));
        var body = pe.GetMethodBody(definition.RelativeVirtualAddress);
        byte[] code = body.GetILBytes() ?? throw new Exception("Owned method body missing.");
        var owner = fixture.GetType("ValueCallCases.Plain");
        var constructor = owner.GetConstructor(new[] { typeof(int), typeof(object) }) ?? throw new Exception("Owned value constructor missing.");
        int[] sites = Enumerable.Range(0, code.Length - 9).Where(i => code[i] == 0x73 && code[i + 5] == 0x81
            && BitConverter.ToInt32(code, i + 1) == constructor.MetadataToken && BitConverter.ToInt32(code, i + 6) == owner.MetadataToken).ToArray();
        if (sites.Length != 1) throw new Exception("Owned newobj/stobj patch shape differs.");
        var section = pe.PEHeaders.SectionHeaders.Single(s => definition.RelativeVirtualAddress >= s.VirtualAddress
            && definition.RelativeVirtualAddress < s.VirtualAddress + Math.Max(s.VirtualSize, s.SizeOfRawData));
        int start = section.PointerToRawData + definition.RelativeVirtualAddress - section.VirtualAddress;
        int codeAt = start + ((image[start] & 3) == 2 ? 1 : (BitConverter.ToUInt16(image, start) >> 12) * 4);
        if (!image.AsSpan(codeAt, code.Length).SequenceEqual(code)) throw new Exception("Owned method byte location differs.");
        image[codeAt + sites[0]] = 0x28;
        for (int i = 0; i < 5; i++) image[codeAt + sites[0] + 5 + i] = 0;
        System.IO.File.WriteAllBytes(args[2], image);
        Console.WriteLine("Authored ReinitializePlain as exact call .ctor on the incoming managed cell.");
    }

    static void ValueCallPlan(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned value-call plan/library.");
        var library = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[3]));
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(args[2]));
        var written = new HashSet<string>();
        foreach (var item in plan.RootElement.EnumerateArray())
        {
            int callerToken = int.Parse(item.GetProperty("caller").GetString(), System.Globalization.NumberStyles.HexNumber);
            var caller = fixture.ManifestModule.ResolveMethod(callerToken);
            foreach (var operand in item.GetProperty("calls").EnumerateArray().Concat(item.GetProperty("constructors").EnumerateArray()).Concat(item.GetProperty("initializers").EnumerateArray()))
            {
                string hex = operand.GetString();
                if (!written.Add(hex)) continue;
                int token = int.Parse(hex, System.Globalization.NumberStyles.HexNumber);
                var target = fixture.ManifestModule.ResolveMethod(token,
                    caller.DeclaringType.GetGenericArguments(), caller.IsGenericMethod ? caller.GetGenericArguments() : Type.EmptyTypes);
                bool constructor = target.IsConstructor;
                Type result = constructor ? target.DeclaringType : ((MethodInfo)target).ReturnType;
                if (!target.DeclaringType.IsValueType || target.IsGenericMethod
                    || result.IsByRef)
                    throw new Exception("Unproved owned value-call target: " + target);
                bool generic = target.ContainsGenericParameters;
                string arguments = generic ? "<T>" : "";
                var parameters = target.GetParameters();
                var declarations = parameters.Select((p, i) => (p.ParameterType.IsByRef ? "ref " : "") + ValueCallType(p.ParameterType) + " p" + i).ToList();
                var values = parameters.Select((p, i) => (p.ParameterType.IsByRef ? "ref " : "") + "p" + i).ToList();
                if (!target.IsStatic && !constructor)
                {
                    declarations.Insert(0, "ref " + ValueCallType(target.DeclaringType) + " receiver");
                    values.Insert(0, "ref receiver");
                }
                string call;
                string typeArguments = !target.DeclaringType.IsGenericType ? "" : generic ? "<T>" : "<" + string.Join(", ", target.DeclaringType.GetGenericArguments().Select(ValueCallType)) + ">";
                if (constructor)
                {
                    string prefix = "private static " + ValueCallType(result) + " __cil_value_newobj_" + hex
                        + arguments + "(" + string.Join(", ", declarations) + ")";
                    if (target.DeclaringType.Assembly == fixture)
                        Console.WriteLine(prefix + " { " + ValueCallType(result) + " storage = default; " + (target.DeclaringType.IsGenericType ? "CellCtor" + typeArguments : "PlainCtor_" + target.MetadataToken.ToString("X8"))
                            + "(ref storage" + (values.Count == 0 ? "" : ", " + string.Join(", ", values)) + "); return storage; }");
                    else Console.WriteLine(prefix + " => new " + ValueCallType(result) + "(" + string.Join(", ", values) + ");");
                    if (target.DeclaringType.Assembly == fixture) {
                        string initialize = target.DeclaringType.IsGenericType ? "CellCtor" + typeArguments : "PlainCtor_" + target.MetadataToken.ToString("X8");
                        Console.WriteLine("private static void __cil_value_ctor_" + hex + arguments
                            + "(ref " + ValueCallType(result) + " receiver" + (declarations.Count == 0 ? "" : ", " + string.Join(", ", declarations)) + ")"
                            + " { " + initialize + "(ref receiver" + (values.Count == 0 ? "" : ", " + string.Join(", ", values)) + "); }");
                    }
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
            if (!item.TryGetProperty("generic_calls", out var genericCalls)) continue;
            foreach (var operand in genericCalls.EnumerateArray())
            {
                string hex = operand.GetString();
                if (!written.Add(hex)) continue;
                int token = int.Parse(hex, System.Globalization.NumberStyles.HexNumber);
                var target = fixture.ManifestModule.ResolveMethod(token,
                    caller.DeclaringType.GetGenericArguments(), caller.IsGenericMethod ? caller.GetGenericArguments() : Type.EmptyTypes) as MethodInfo
                    ?? throw new Exception("Owned generic value-call target missing.");
                var definition = library.GetType("ValueCallLibrary.Cell`1") ?? throw new Exception("Owned foreign cell missing.");
                var declarations = definition.GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly)
                    .Where(m => m.Name == "Exchange").ToArray();
                if (declarations.Length != 1 || !target.IsGenericMethod || target.IsStatic || target.IsVirtual
                    || target.DeclaringType.GetGenericTypeDefinition() != definition
                    || target.GetGenericMethodDefinition().MetadataToken != declarations[0].MetadataToken
                    || target.GetGenericArguments().Length != 1 || target.ReturnType.IsByRef)
                    throw new Exception("Unproved owned generic value-call target: " + target);
                var variables = target.DeclaringType.GetGenericArguments().Concat(target.GetGenericArguments())
                    .Where(t => t.IsGenericParameter).Distinct().ToArray();
                if (variables.Length > 1 || variables.Any(t => t.Name != "T"))
                    throw new Exception("Unexpected owned generic value-call scope.");
                var parameters = target.GetParameters();
                var argspec = parameters.Select((p, i) => (p.ParameterType.IsByRef ? "ref " : "") + ValueCallType(p.ParameterType) + " p" + i);
                var values = parameters.Select((p, i) => (p.ParameterType.IsByRef ? "ref " : "") + "p" + i);
                Console.WriteLine("private static " + ValueCallType(target.ReturnType) + " __cil_methodspec_call_" + hex
                    + (variables.Length == 0 ? "" : "<T>") + "(ref " + ValueCallType(target.DeclaringType) + " receiver, "
                    + string.Join(", ", argspec) + ") => receiver.Exchange<" + ValueCallType(target.GetGenericArguments()[0])
                    + ">(" + string.Join(", ", values) + ");");
            }
            if (!item.TryGetProperty("field_addresses", out var fieldAddresses)) continue;
            foreach (var operand in fieldAddresses.EnumerateArray())
            {
                string hex = operand.GetString();
                if (!written.Add(hex)) continue;
                int token = int.Parse(hex, System.Globalization.NumberStyles.HexNumber);
                var field = fixture.ManifestModule.ResolveField(token,
                    caller.DeclaringType.GetGenericArguments(), caller.IsGenericMethod ? caller.GetGenericArguments() : Type.EmptyTypes)
                    ?? throw new Exception("Owned field address target missing.");
                var definition = library.GetType("ValueCallLibrary.Cell`1") ?? throw new Exception("Owned foreign cell missing.");
                var count = definition.GetField("Count") ?? throw new Exception("Owned field missing.");
                if (field.IsStatic || field.IsInitOnly || field.FieldType != typeof(int)
                    || !field.DeclaringType.IsGenericType || field.DeclaringType.ContainsGenericParameters
                    || field.DeclaringType.GetGenericTypeDefinition() != definition || field.MetadataToken != count.MetadataToken)
                    throw new Exception("Unproved owned field address target: " + field);
                Console.WriteLine("private static ref int __cil_ldflda_" + hex + "(ref " + ValueCallType(field.DeclaringType)
                    + " receiver) => ref receiver.Count;");
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
            .Where(m => !m.Name.StartsWith("Cell", StringComparison.Ordinal) && !m.Name.StartsWith("PlainCtor_", StringComparison.Ordinal)
                && m.Name != "ProbeArgument" && m.Name != "ProbeArgumentTag").ToArray();
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
                    object oldPlain = definition.Name == "NewPlainInto" || definition.Name == "ReinitializePlain"
                        ? beforeArgs[0].GetType().GetField("Value").GetValue(beforeArgs[0]) : null;
                    Reset(next == 0 ? 0 : 2);
                    Outcome before = Capture(() => ManagedInvoker(original)(beforeArgs), new List<State>());
                    if (new[] { "NewPlain", "NewPlainInto", "NewPlainOrder", "NewPlainDelegated", "ReinitializePlain" }.Contains(definition.Name)) {
                        bool destination = definition.Name == "NewPlainInto" || definition.Name == "ReinitializePlain";
                        bool ordered = definition.Name == "NewPlainOrder", delegated = definition.Name == "NewPlainDelegated";
                        int value = (int)beforeArgs[destination ? 1 : 0];
                        object tag = delegated ? null : beforeArgs[destination ? 2 : 1];
                        bool argumentFailed = ordered && tag == null;
                        Type expectedError = argumentFailed ? typeof(NullReferenceException) : next == 0 ? typeof(DivideByZeroException) : null;
                        int expectedTrace = argumentFailed ? 312 : ordered ? 3123 : delegated && expectedError == null ? 334 : 33;
                        Type probe = fixture.GetType("ValueCallCases.Probe");
                        if (before.Error != expectedError || (int)probe.GetField("Trace").GetValue(null) != expectedTrace
                            || (int)probe.GetField("Entries").GetValue(null) != (argumentFailed ? 7 : 8))
                            throw new Exception("Owned plain constructor expectation differs.");
                        object storage = destination ? beforeArgs[0].GetType().GetField("Value").GetValue(beforeArgs[0]) : before.Value;
                        Type plain = fixture.GetType("ValueCallCases.Plain");
                        if (expectedError == null) {
                            if ((int)plain.GetField("Value").GetValue(storage) != value || (int)plain.GetField("Count").GetValue(storage) != 50
                                || !ReferenceEquals(plain.GetField("Tag").GetValue(storage), tag)) throw new Exception("Owned completed value differs.");
                        } else if (destination) {
                            byte[] il = original.GetMethodBody().GetILAsByteArray();
                            int ctor = plain.GetConstructor(new[] { typeof(int), typeof(object) }).MetadataToken;
                            byte[] call = new[] { (byte)0x28 }.Concat(BitConverter.GetBytes(ctor)).ToArray();
                            bool acquired = Enumerable.Range(0, il.Length - 4).Any(i => il.AsSpan(i, 5).SequenceEqual(call));
                            if (acquired) {
                                if ((int)plain.GetField("Value").GetValue(storage) != value
                                    || !Equals(plain.GetField("Count").GetValue(storage), plain.GetField("Count").GetValue(oldPlain))
                                    || !ReferenceEquals(plain.GetField("Tag").GetValue(storage), plain.GetField("Tag").GetValue(oldPlain)))
                                    throw new Exception("Owned acquired value constructor writes differ.");
                            } else if (Describe(storage) != Describe(oldPlain)) throw new Exception("Owned failed construction changed destination.");
                        }
                    }
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
