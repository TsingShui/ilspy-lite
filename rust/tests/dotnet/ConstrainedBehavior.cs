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
using System.Reflection.Emit;
using System.Runtime.Loader;
using System.Text.Json;

internal static partial class GenericReferenceRoundtrip
{
    static string ConstrainedParameter(Type type, string name) =>
        (type.IsByRef ? "ref " : "") + ValueCallType(type) + " " + name;

    static Type[] ConstrainedTypes(MethodInfo method)
    {
        if (!method.IsGenericMethodDefinition && !method.DeclaringType.ContainsGenericParameters) return new[] { typeof(void) };
        if (method.Name == "Text" || method.Name == "MaybeText") return new[] { typeof(int), typeof(string), typeof(int?), typeof(object),
            fixture.GetType("ConstrainedCases.Plain"), fixture.GetType("ConstrainedCases.Counter"),
            fixture.GetType("ConstrainedCases.CounterClass"), fixture.GetType("ConstrainedCases.DerivedCounter") };
        return new[] { fixture.GetType("ConstrainedCases.Counter"), fixture.GetType("ConstrainedCases.CounterClass"),
            fixture.GetType("ConstrainedCases.DerivedCounter") };
    }

    static void ConstrainedPlan(string path)
    {
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(path));
        var results = new List<object>();
        int sequence = 0;
        foreach (var row in plan.RootElement.EnumerateArray())
        {
            int token = row.GetProperty("token").GetInt32();
            var definition = (MethodInfo)fixture.ManifestModule.ResolveMethod(token);
            foreach (var argument in ConstrainedTypes(definition))
            {
                if (argument == null) throw new Exception("Owned constrained type missing.");
                var typeArguments = definition.DeclaringType.IsGenericTypeDefinition ? new[] { argument } : Type.EmptyTypes;
                var methodArguments = definition.IsGenericMethodDefinition ? new[] { argument } : Type.EmptyTypes;
                MethodInfo original = typeArguments.Length == 0 ? definition : definition.DeclaringType.MakeGenericType(typeArguments)
                    .GetMethods().Single(m => m.MetadataToken == token);
                if (methodArguments.Length != 0) original = original.MakeGenericMethod(methodArguments);
                string header = "public static " + ValueCallType(original.ReturnType) + " " + original.Name + "("
                    + string.Join(", ", original.GetParameters().Select(p => ConstrainedParameter(p.ParameterType, p.Name))) + ")";
                var aliases = new List<string>();
                foreach (var alias in row.GetProperty("aliases").EnumerateArray())
                {
                    byte[] bytes = Convert.FromHexString(alias.GetProperty("bytes").GetString());
                    int position = 0;
                    Type type = Decode(bytes, ref position, typeArguments, methodArguments);
                    if (position != bytes.Length || type.ContainsGenericParameters) throw new Exception("Incomplete constrained alias.");
                    aliases.Add("using " + alias.GetProperty("alias").GetString() + " = " + ValueCallType(type) + ";");
                }
                var helpers = new List<string>();
                int operation = 0;
                foreach (var call in row.GetProperty("calls").EnumerateArray())
                {
                    int constraintToken = call.GetProperty("constraint").GetInt32();
                    int methodToken = call.GetProperty("method").GetInt32();
                    Type constraint = fixture.ManifestModule.ResolveType(constraintToken, typeArguments, methodArguments);
                    var target = (MethodInfo)fixture.ManifestModule.ResolveMethod(methodToken, typeArguments, methodArguments);
                    if (target.IsStatic || target.IsGenericMethod || target.ReturnType.IsByRef)
                        throw new Exception("Unproved owned constrained target: " + target);
                    Type[] parameters = target.GetParameters().Select(p => p.ParameterType).Prepend(constraint.MakeByRefType()).ToArray();
                    string name = "__cil_constrained_" + constraintToken.ToString("X8") + "_" + methodToken.ToString("X8");
                    string id = "Operation" + operation++;
                    string declarations = string.Join(", ", parameters.Select((p, i) => ConstrainedParameter(p, "p" + i)));
                    string operands = string.Join(", ", parameters.Select((p, i) => (p.IsByRef ? "ref " : "") + "p" + i));
                    string result = ValueCallType(target.ReturnType);
                    string shape = string.Join(", ", parameters.Select(p => "typeof(" + ValueCallType(p) + ")" + (p.IsByRef ? ".MakeByRefType()" : "")));
                    string args = string.Join(", ", methodArguments.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    string ownerArgs = string.Join(", ", typeArguments.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    string support = "private delegate " + result + " " + id + "Delegate(" + declarations + ");"
                        + " private static readonly System.Lazy<" + id + "Delegate> " + id + " = new System.Lazy<" + id + "Delegate>(() => {"
                        + " var module = typeof(global::ConstrainedCases.Ops).Module;"
                        + " var target = (System.Reflection.MethodInfo)module.ResolveMethod(" + methodToken + ", new System.Type[]{" + ownerArgs + "}, new System.Type[]{" + args + "});"
                        + " var method = new System.Reflection.Emit.DynamicMethod(\"owned-constrained-leaf\", typeof(" + result + "), new System.Type[]{" + shape + "}, module, true);"
                        + " var il = method.GetILGenerator();";
                    for (int i = 0; i < parameters.Length; i++) support += "il.Emit(System.Reflection.Emit.OpCodes.Ldarg, (short)" + i + ");";
                    support += "il.Emit(System.Reflection.Emit.OpCodes.Constrained, typeof(" + ValueCallType(constraint) + "));"
                        + "il.Emit(System.Reflection.Emit.OpCodes.Callvirt, target); il.Emit(System.Reflection.Emit.OpCodes.Ret);"
                        + "return (" + id + "Delegate)method.CreateDelegate(typeof(" + id + "Delegate)); });"
                        + "private static " + result + " " + name + "(" + declarations + ")"
                        + (target.ReturnType == typeof(void) ? " { " + id + ".Value(" + operands + "); }" : " => " + id + ".Value(" + operands + ");");
                    helpers.Add(support);
                }
                foreach (var call in row.GetProperty("methodspecs").EnumerateArray())
                {
                    int specToken = call.GetInt32();
                    var target = (MethodInfo)fixture.ManifestModule.ResolveMethod(specToken, typeArguments, methodArguments);
                    if (!target.IsStatic || target.DeclaringType.FullName != "ConstrainedCases.Ledger" || target.Name != "Replace"
                        || !target.GetGenericArguments().SequenceEqual(methodArguments))
                        throw new Exception("Unexpected owned constrained support call.");
                    helpers.Add("private static int __cil_methodspec_call_" + specToken.ToString("X8") + "("
                        + string.Join(", ", target.GetParameters().Select((p, i) => ConstrainedParameter(p.ParameterType, "p" + i)))
                        + ") => Replace(ref p0, p1, p2);");
                }
                foreach (var call in row.GetProperty("directs").EnumerateArray())
                {
                    int targetToken = call.GetInt32();
                    var target = (MethodInfo)fixture.ManifestModule.ResolveMethod(targetToken);
                    if (target.IsVirtual && !target.IsFinal || target.IsStatic || !target.DeclaringType.IsValueType || target.Name != "Step"
                        || target.ReturnType != typeof(int) || target.GetParameters().Single().ParameterType != typeof(int))
                        throw new Exception("Unexpected owned direct value target.");
                    helpers.Add("private static int __cil_direct_" + targetToken.ToString("X8") + "(ref "
                        + ValueCallType(target.DeclaringType) + " receiver, int delta) => receiver.Step(delta);");
                }
                results.Add(new { carrier = "Constrained" + sequence++, token, header, aliases, helpers,
                    argument = methodArguments.Length == 0 ? null : argument.AssemblyQualifiedName,
                    type_argument = typeArguments.Length == 0 ? null : argument.AssemblyQualifiedName });
            }
        }
        Console.WriteLine(JsonSerializer.Serialize(results));
    }

    static void ConstrainedCheck(string carrierPath, string planPath)
    {
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(carrierPath));
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(planPath));
        FieldInfo trace = fixture.GetType("ConstrainedCases.Ledger").GetField("Trace");
        string first = new string("first".ToCharArray()), second = new string("second".ToCharArray());
        int checks = 0, selections = 0;
        foreach (var row in plan.RootElement.EnumerateArray())
        {
            var original = (MethodInfo)fixture.ManifestModule.ResolveMethod(row.GetProperty("token").GetInt32());
            if (row.GetProperty("type_argument").ValueKind != JsonValueKind.Null)
                original = original.DeclaringType.MakeGenericType(Type.GetType(row.GetProperty("type_argument").GetString(), true))
                    .GetMethods().Single(m => m.MetadataToken == original.MetadataToken);
            if (original.IsGenericMethodDefinition)
                original = original.MakeGenericMethod(Type.GetType(row.GetProperty("argument").GetString(), true));
            var lowered = generated.GetType(row.GetProperty("carrier").GetString() + ".Recovered").GetMethod(original.Name);
            if (lowered == null || original.ReturnType != lowered.ReturnType ||
                !original.GetParameters().Select(p => p.ParameterType).SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                throw new Exception("Constrained ABI differs.");
            selections++;
            foreach (int initial in new[] { 0, 1, 2 })
            foreach (int next in new[] { 0, 1, 2 })
            foreach (bool alias in new[] { false, true })
            {
                List<object> tracked = null;
                object Sample(Type type, int variant)
                {
                    if (type.IsByRef)
                    {
                        Type cellType = typeof(ManagedArgument<>).MakeGenericType(type.GetElementType());
                        object cell = Activator.CreateInstance(cellType);
                        cellType.GetField("Value").SetValue(cell, Sample(type.GetElementType(), variant));
                        return cell;
                    }
                    if (type == typeof(int)) return variant == 0 ? 0 : variant == 1 ? int.MinValue : 17;
                    if (type == typeof(bool)) return variant != 0;
                    if (type == typeof(uint[])) return variant == 0 ? null : variant == 1 ? Array.Empty<uint>() : new[] { 0xffffffffu, 17u, 3u };
                    if (type == typeof(string)) return variant == 0 ? null : variant == 1 ? first : second;
                    if (type == typeof(object)) return variant == 0 ? null : first;
                    Type underlying = Nullable.GetUnderlyingType(type);
                    if (underlying != null) return variant == 0 ? null : Sample(underlying, variant);
                    if (!type.IsValueType && variant == 0) return null;
                    object value = Activator.CreateInstance(type);
                    if (!type.IsValueType) tracked.Add(value);
                    foreach (var field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                        field.SetValue(value, Sample(field.FieldType, variant));
                    return value;
                }
                string Describe(object value)
                {
                    if (value == null) return "null";
                    if (value is string text) return ReferenceEquals(value, first) ? "first" : ReferenceEquals(value, second) ? "second" : "text:" + text;
                    Type type = value.GetType();
                    if (value is uint[] array) return "[" + string.Join(",", array) + "]";
                    if (type.IsPrimitive) return type.FullName + ":" + value;
                    int identity = tracked.FindIndex(item => ReferenceEquals(item, value));
                    return (identity < 0 ? "" : "ref" + identity + ":") + type.FullName + "{"
                        + string.Join(",", type.GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name)
                            .Select(f => f.Name + "=" + Describe(f.GetValue(value)))) + "}";
                }
                (Outcome outcome, string state) Execute(MethodInfo method)
                {
                    tracked = new List<object>();
                    var parameters = method.GetParameters();
                    object[] arguments = parameters.Select((p, i) => p.Name == "delta" ? (object)(next == 0 ? 0 : next == 1 ? 2 : -2)
                        : p.Name == "index" ? (object)(next == 0 ? -1 : next == 1 ? 0 : 2)
                        : p.Name == "first" ? (object)(original.Name == "ArrayCarry" ? alias : initial != 0) : Sample(p.ParameterType, i == 0 ? initial : next)).ToArray();
                    if (alias && (original.Name == "Conditional" || original.Name == "Local")) arguments[2] = arguments[1];
                    if (alias && original.Name == "Slot")
                        arguments[1] = arguments[0].GetType().GetField("Value").GetValue(arguments[0]);
                    trace.SetValue(null, 7);
                    Outcome result = Capture(() => ManagedInvoker(method)(arguments), new List<State>());
                    string state = Describe(result.Value) + "|" + string.Join("|", arguments.Select(Describe))
                        + "|objects:" + string.Join("|", tracked.Select(Describe)) + "|trace:" + trace.GetValue(null);
                    return (result, state);
                }
                var before = Execute(original);
                var after = Execute(lowered);
                if (before.outcome.Error != after.outcome.Error || before.state != after.state)
                    throw new Exception("Recovered constrained behavior differs: " + original.Name + " expected=" + before.outcome.Error + "/" + before.state
                        + " actual=" + after.outcome.Error + "/" + after.state);
                checks++;
            }
        }
        if (selections == 0) throw new Exception("Constrained selection empty.");
        Console.WriteLine("Constrained behavior: " + checks + " comparisons, " + selections
            + " closed selections; location joins, receiver slots, value copies, dispatch, argument effects and exceptions match.");
    }
}
