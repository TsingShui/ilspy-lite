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
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.Loader;
using System.Text.Json;

internal static partial class GenericReferenceRoundtrip
{
    static Assembly ReferenceCallLibrary(string path)
    {
        AssemblyName identity = AssemblyName.GetAssemblyName(path);
        Assembly library = null;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
            name.FullName == identity.FullName ? library ??= context.LoadFromAssemblyPath(path) : null;
        return library ??= AssemblyLoadContext.Default.LoadFromAssemblyPath(path);
    }

    static void ReferenceCallPlan(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned leaf plan and library.");
        if (args[3] != "-") ReferenceCallLibrary(Path.GetFullPath(args[3]));
        using var input = JsonDocument.Parse(File.ReadAllText(args[2]));
        var aliases = input.RootElement.GetProperty("aliases").EnumerateArray().Select(row =>
        {
            int token = Convert.ToInt32(row.GetString(), 16);
            return "using __cil_type_" + token.ToString("X8") + " = "
                + TypeSource(fixture.ManifestModule.ResolveType(token)) + ";";
        }).ToArray();
        var helpers = new List<string>();
        foreach (var row in input.RootElement.GetProperty("calls").EnumerateArray())
        {
            int token = Convert.ToInt32(row.GetProperty("token").GetString(), 16);
            string operation = row.GetProperty("operation").GetString();
            int table = token & unchecked((int)0xff000000);
            if ((table != 0x0a000000 && table != 0x2b000000)
                || !(operation == "callvirt" || operation == "reference_call"
                     || operation == "memberref_call" || operation == "methodspec_call"))
                throw new Exception("Owned leaf plan is not an exact MemberRef or MethodSpec call.");
            MethodInfo target = fixture.ManifestModule.ResolveMethod(token) as MethodInfo
                ?? throw new Exception("Owned leaf is not an ordinary method.");
            if (target.ContainsGenericParameters || target.ReturnType.IsByRef
                || target.GetParameters().Any(p => p.ParameterType.IsByRef))
                throw new Exception("Unsupported owned leaf signature.");
            var parameters = new List<string>();
            var arguments = new List<string>();
            if (!target.IsStatic) parameters.Add(TypeSource(target.DeclaringType) + " receiver");
            foreach (ParameterInfo parameter in target.GetParameters())
            {
                parameters.Add(TypeSource(parameter.ParameterType) + " a" + parameter.Position);
                arguments.Add("a" + parameter.Position);
            }
            string key = operation + "_" + token.ToString("X8");
            string cache = "__leaf_" + key;
            string invoke = "(" + cache + " ??= __CreateLeaf(" + token + ", "
                + (operation == "callvirt" ? "true" : "false") + "))("
                + (target.IsStatic ? "null" : "receiver") + ", new object[] {"
                + string.Join(", ", arguments) + "})";
            helpers.Add("static System.Func<object, object[], object> " + cache + ";\n"
                + "static " + (target.ReturnType == typeof(void) ? "void" : TypeSource(target.ReturnType)) + " __cil_" + key
                + "(" + string.Join(", ", parameters) + ") { "
                + (target.ReturnType == typeof(void) ? invoke + ";" : "return (" + TypeSource(target.ReturnType) + ")" + invoke + ";") + " }");
        }
        const string support = """
static System.Func<object, object[], object> __CreateLeaf(int token, bool virtualCall)
{
    var module = typeof(ReferenceCallCases.Ops).Assembly.ManifestModule;
    var method = (System.Reflection.MethodInfo)module.ResolveMethod(token);
    var body = new System.Reflection.Emit.DynamicMethod("ExactLeaf_" + token,
        typeof(object), new[] { typeof(object), typeof(object[]) }, typeof(Recovered).Module, true);
    var il = body.GetILGenerator();
    if (!method.IsStatic) { il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0); il.Emit(System.Reflection.Emit.OpCodes.Castclass, method.DeclaringType); }
    var parameters = method.GetParameters();
    for (int index = 0; index < parameters.Length; index++)
    {
        il.Emit(System.Reflection.Emit.OpCodes.Ldarg_1); il.Emit(System.Reflection.Emit.OpCodes.Ldc_I4, index);
        il.Emit(System.Reflection.Emit.OpCodes.Ldelem_Ref);
        var type = parameters[index].ParameterType;
        il.Emit(type.IsValueType ? System.Reflection.Emit.OpCodes.Unbox_Any : System.Reflection.Emit.OpCodes.Castclass, type);
    }
    il.Emit(virtualCall ? System.Reflection.Emit.OpCodes.Callvirt : System.Reflection.Emit.OpCodes.Call, method);
    if (method.ReturnType == typeof(void)) il.Emit(System.Reflection.Emit.OpCodes.Ldnull);
    else if (method.ReturnType.IsValueType) il.Emit(System.Reflection.Emit.OpCodes.Box, method.ReturnType);
    il.Emit(System.Reflection.Emit.OpCodes.Ret);
    return (System.Func<object, object[], object>)body.CreateDelegate(typeof(System.Func<object, object[], object>));
}
""";
        Console.WriteLine(JsonSerializer.Serialize(new { aliases, helpers, support }));
    }

    static void ReferenceCallMain(string[] args)
    {
        if (args.Length != 4) throw new Exception("Expected owned reference-call fixture, carrier and library.");
        Assembly library = ReferenceCallLibrary(Path.GetFullPath(args[3]));
        Type ops = fixture.GetType("ReferenceCallCases.Ops") ?? throw new Exception("Owned Ops missing.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ReferenceCallCases.Recovered") ?? throw new Exception("Owned carrier missing.");
        object Make(Type type)
        {
            object result = Activator.CreateInstance(type);
            foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                field.SetValue(result, field.FieldType == typeof(int) ? (object)7
                    : field.FieldType == typeof(string) ? "initial" : throw new Exception("Unexpected owned field."));
            return result;
        }
        object[] Candidates(Type type)
        {
            if (type.IsByRef)
            {
                Type element = type.GetElementType();
                Type cell = typeof(ManagedArgument<>).MakeGenericType(element);
                return Candidates(element).Select(value =>
                {
                    object box = Activator.CreateInstance(cell);
                    cell.GetField("Value").SetValue(box, value);
                    return box;
                }).ToArray();
            }
            if (type == typeof(int)) return new object[] { 0, 2, -2 };
            if (type == typeof(string)) return new object[] { null, "", "next" };
            if (type == typeof(int[])) return new object[] { null, Array.Empty<int>(), new[] { 1, -2, 7 } };
            if (type == typeof(object))
                return new[] { null, Make(library.GetType("ReferenceCalls.Box`1").MakeGenericType(typeof(int))),
                    Make(library.GetType("ReferenceCalls.ExplicitBox`1").MakeGenericType(typeof(int))), new object(), "text" };
            var values = new List<object> { null };
            foreach (Type definition in library.GetTypes().Where(t => !t.IsAbstract && !t.IsInterface))
            {
                Type candidate = definition;
                if (candidate.IsGenericTypeDefinition)
                {
                    if (!type.IsGenericType || candidate.GetGenericArguments().Length != type.GenericTypeArguments.Length) continue;
                    candidate = candidate.MakeGenericType(type.GenericTypeArguments);
                }
                if (candidate.ContainsGenericParameters || !type.IsAssignableFrom(candidate)) continue;
                values.Add(Make(candidate));
            }
            if (values.Count == 1) throw new Exception("No owned nonnull receiver for " + type);
            return values.ToArray();
        }
        object Copy(object value)
        {
            if (value == null || value is string || value.GetType().IsPrimitive || value.GetType() == typeof(object)) return value;
            if (value is Array array) return array.Clone();
            object result = Activator.CreateInstance(value.GetType());
            foreach (FieldInfo field in value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance))
                field.SetValue(result, Copy(field.GetValue(value)));
            return result;
        }
        string Describe(object value, object[] arguments)
        {
            if (value == null) return "null";
            if (value is string || value.GetType().IsPrimitive) return value.GetType().FullName + ":" + value;
            for (int index = 0; index < arguments.Length; index++)
                if (ReferenceEquals(value, arguments[index])) return "arg:" + index;
            if (value is Array array) return "array:[" + string.Join(",", array.Cast<object>()) + "]";
            return value.GetType().FullName + "{" + string.Join(",",
                value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name)
                    .Select(field => field.Name + "=" + Describe(field.GetValue(value), Array.Empty<object>()))) + "}";
        }
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (selected.Length == 0) throw new Exception("Owned reference-call selection is empty.");
        int comparisons = 0;
        foreach (MethodInfo lowered in selected.OrderBy(m => m.Name))
        {
            MethodInfo original = ops.GetMethod(lowered.Name) ?? throw new Exception("Owned counterpart missing.");
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (original.ReturnType != lowered.ReturnType
                || !parameters.SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                throw new Exception("Recovered reference-call signature differs: " + original.Name);
            var cases = new List<object[]> { Array.Empty<object>() };
            foreach (Type parameter in parameters)
                cases = cases.SelectMany(prefix => Candidates(parameter).Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
            foreach (object[] arguments in cases)
            {
                object[] beforeArgs = arguments.Select(Copy).ToArray();
                object[] afterArgs = arguments.Select(Copy).ToArray();
                bool byref = parameters.Any(type => type.IsByRef);
                Outcome before = Capture(() => byref ? ManagedInvoker(original)(beforeArgs) : OriginalCall(original)(null, beforeArgs), new List<State>());
                Outcome after = Capture(() => byref ? ManagedInvoker(lowered)(afterArgs) : lowered.Invoke(null, afterArgs), new List<State>());
                string StateOf(object[] values) => string.Join("|", values.Select(value => Describe(value, Array.Empty<object>())));
                if (before.Error != after.Error || Describe(before.Value, beforeArgs) != Describe(after.Value, afterArgs)
                    || StateOf(beforeArgs) != StateOf(afterArgs))
                    throw new Exception("Recovered reference call differs: " + original.Name
                        + " expected=" + (before.Error?.Name ?? Describe(before.Value, beforeArgs))
                        + " actual=" + (after.Error?.Name ?? Describe(after.Value, afterArgs))
                        + " state=" + StateOf(beforeArgs) + "/" + StateOf(afterArgs));
                comparisons++;
            }
        }
        Console.WriteLine("Reference call behavior: " + comparisons + " comparisons, " + selected.Length
            + " methods; class/interface/indexed dispatch, generic owners, argument effects, state, casts and array identity match.");
    }
}
