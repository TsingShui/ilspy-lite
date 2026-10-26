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
    static Assembly StaticFieldLoadLibrary()
    {
        string name = fixture.GetName().Name;
        if (name != "static-field-release" && name != "static-field-debug") throw new Exception("Unexpected owned static fixture.");
        string config = name.EndsWith("-debug") ? "debug" : "release";
        return AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.Combine(System.IO.Path.GetDirectoryName(fixture.Location), "static-field-library-" + config + ".bin"));
    }

    static void StaticFieldPlan(string path)
    {
        StaticFieldLoadLibrary();
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(path));
        var results = new List<object>();
        int sequence = 0;
        foreach (var row in plan.RootElement.EnumerateArray())
        {
            int token = row.GetProperty("token").GetInt32();
            var definition = (MethodInfo)fixture.ManifestModule.ResolveMethod(token);
            var arguments = definition.IsGenericMethodDefinition || definition.DeclaringType.IsGenericTypeDefinition
                ? new[] { typeof(int), typeof(string), typeof(object), typeof(int?), fixture.GetType("StaticFieldCases.Pair") ?? throw new Exception("Owned pair missing.") } : new[] { typeof(void) };
            if (definition.Name.StartsWith("Init", StringComparison.Ordinal)) arguments = new[] { typeof(int), typeof(string) };
            foreach (Type ownerArgument in definition.DeclaringType.IsGenericTypeDefinition ? arguments : new[] { typeof(void) })
            foreach (Type methodArgument in definition.IsGenericMethodDefinition ? arguments : new[] { typeof(void) })
            {
                Type[] ownerArgs = definition.DeclaringType.IsGenericTypeDefinition ? new[] { ownerArgument } : Type.EmptyTypes;
                Type[] methodArgs = definition.IsGenericMethodDefinition ? new[] { methodArgument } : Type.EmptyTypes;
                MethodInfo original = ownerArgs.Length == 0 ? definition : definition.DeclaringType.MakeGenericType(ownerArgs)
                    .GetMethods().Single(m => m.MetadataToken == token);
                if (methodArgs.Length != 0) original = original.MakeGenericMethod(methodArgs);
                string header = "public static " + ValueCallType(original.ReturnType) + " " + original.Name + "("
                    + string.Join(", ", original.GetParameters().Select(p => ConstrainedParameter(p.ParameterType, p.Name))) + ")";
                var aliases = new List<string>();
                foreach (var alias in row.GetProperty("aliases").EnumerateArray())
                {
                    byte[] bytes = Convert.FromHexString(alias.GetProperty("bytes").GetString());
                    int position = 0;
                    Type type = Decode(bytes, ref position, ownerArgs, methodArgs);
                    if (position != bytes.Length || type.ContainsGenericParameters) throw new Exception("Incomplete static field alias.");
                    aliases.Add("using " + alias.GetProperty("alias").GetString() + " = " + ValueCallType(type) + ";");
                }
                var helpers = new List<string>();
                int operation = 0;
                foreach (var leaf in row.GetProperty("fields").EnumerateArray())
                {
                    int fieldToken = leaf.GetProperty("token").GetInt32();
                    string op = leaf.GetProperty("op").GetString();
                    var field = fixture.ManifestModule.ResolveField(fieldToken, ownerArgs, methodArgs);
                    if (!field.IsStatic || field.FieldType.ContainsGenericParameters) throw new Exception("Owned static field not closed: " + field);
                    bool store = op == "stsfld", address = op == "ldsflda";
                    string type = ValueCallType(field.FieldType), id = "FieldOperation" + operation++;
                    string returned = store ? "void" : (address ? "ref " : "") + type;
                    string parameters = store ? type + " value" : "";
                    string shape = store ? "new System.Type[]{typeof(" + type + ")}" : "System.Type.EmptyTypes";
                    string resultType = store ? "typeof(void)" : "typeof(" + type + ")" + (address ? ".MakeByRefType()" : "");
                    string ownerTypes = string.Join(", ", ownerArgs.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    string methodTypes = string.Join(", ", methodArgs.Select(t => "typeof(" + ValueCallType(t) + ")"));
                    string opcode = store ? "Stsfld" : address ? "Ldsflda" : "Ldsfld";
                    string helper = "private delegate " + returned + " " + id + "Delegate(" + parameters + ");"
                        + "private static readonly System.Lazy<" + id + "Delegate> " + id + " = new System.Lazy<" + id + "Delegate>(() => {"
                        + "var module = typeof(global::StaticFieldCases.Ops).Module;"
                        + "var target = module.ResolveField(" + fieldToken + ", new System.Type[]{" + ownerTypes + "}, new System.Type[]{" + methodTypes + "});"
                        + "var method = new System.Reflection.Emit.DynamicMethod(\"owned-static-field-leaf\", " + resultType + ", " + shape + ", module, true);"
                        + "var il = method.GetILGenerator();"
                        + (store ? "il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);" : "")
                        + "il.Emit(System.Reflection.Emit.OpCodes." + opcode + ", target);il.Emit(System.Reflection.Emit.OpCodes.Ret);"
                        + "return (" + id + "Delegate)method.CreateDelegate(typeof(" + id + "Delegate)); });"
                        + "private static " + returned + " __cil_" + op + "_" + fieldToken.ToString("X8") + "(" + parameters + ")"
                        + (store ? " { " + id + ".Value(value); }" : " => " + (address ? "ref " : "") + id + ".Value();");
                    helpers.Add(helper);
                }
                results.Add(new { carrier = "StaticField" + sequence++, token, header, aliases, helpers,
                    argument = methodArgs.Length == 0 ? null : methodArgument.AssemblyQualifiedName,
                    type_argument = ownerArgs.Length == 0 ? null : ownerArgument.AssemblyQualifiedName });
            }
        }
        Console.WriteLine(JsonSerializer.Serialize(results));
    }

    static void StaticFieldOnce(string request)
    {
        StaticFieldLoadLibrary();
        using var json = JsonDocument.Parse(System.IO.File.ReadAllText(request));
        var data = json.RootElement;
        var row = data.GetProperty("row");
        var original = (MethodInfo)fixture.ManifestModule.ResolveMethod(row.GetProperty("token").GetInt32());
        Type argument = Type.GetType(row.GetProperty("argument").GetString(), true);
        original = original.MakeGenericMethod(argument);
        Type ops = fixture.GetType("StaticFieldCases.Ops");
        FieldInfo trace = ops.GetField("Trace"), attempts = ops.GetField("Attempts"), fail = ops.GetField("FailInit");
        trace.SetValue(null, 0); attempts.SetValue(null, 0);
        bool failure = data.GetProperty("fail").GetBoolean();
        fail.SetValue(null, failure);
        MethodInfo method = original;
        if (data.GetProperty("recovered").GetBoolean()) {
            var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(data.GetProperty("binary").GetString());
            method = generated.GetType(row.GetProperty("carrier").GetString() + ".Recovered").GetMethod(original.Name);
        }
        if (original.ReturnType != method.ReturnType || !original.GetParameters().Select(p => p.ParameterType)
            .SequenceEqual(method.GetParameters().Select(p => p.ParameterType))) throw new Exception("Initialization ABI differs.");
        object[] values = method.GetParameters().Select(_ => (object)data.GetProperty("value").GetInt32()).ToArray();
        string Describe(Exception error) => error == null ? null : error.GetType().FullName + "|" +
            (error is TypeInitializationException ? "" : error.Message) + "|" + Describe(error.InnerException);
        var rows = new List<object>();
        for (int i = 0; i < 2; i++) {
            object result = null; Exception error = null;
            try { result = ManagedInvoker(method)(values); } catch (Exception e) { error = e; }
            int count = (int)attempts.GetValue(null);
            object stored = count == 0 ? "uninitialized" : failure ? "poisoned" :
                fixture.GetType("StaticFieldCases.Initialized\u00601").MakeGenericType(argument).GetField("Value").GetValue(null);
            rows.Add(new { result, error = Describe(error), trace = trace.GetValue(null), attempts = count, stored });
        }
        Console.WriteLine(JsonSerializer.Serialize(rows));
    }

    static void StaticFieldCheck(string path, string planPath, bool threadsOnly = false)
    {
        var library = StaticFieldLoadLibrary();
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(path));
        using var plan = JsonDocument.Parse(System.IO.File.ReadAllText(planPath));
        Type slots = fixture.GetType("StaticFieldCases.Slots\u00601") ?? throw new Exception("Owned generic storage missing.");
        Type pair = fixture.GetType("StaticFieldCases.Pair") ?? throw new Exception("Owned pair missing.");
        Type[] types = { typeof(int), typeof(string), typeof(object), typeof(int?), pair };
        string first = new string("first".ToCharArray()), second = new string("second".ToCharArray());
        object shared = new object();
        object Sample(Type type, int variant)
        {
            if (type == pair) {
                object value = Activator.CreateInstance(type);
                type.GetField("Count").SetValue(value, Sample(typeof(int), variant));
                type.GetField("Tag").SetValue(value, Sample(typeof(object), variant));
                return value;
            }
            if (Nullable.GetUnderlyingType(type) != null) return variant == 0 ? null : Sample(Nullable.GetUnderlyingType(type), variant);
            return type == typeof(int) ? (object)(variant == 0 ? int.MinValue : variant == 1 ? 17 : -1)
                : type == typeof(string) ? variant == 0 ? null : variant == 1 ? first : second
                : variant == 0 ? null : variant == 1 ? shared : second;
        }
        string Describe(object value) => value == null ? "null" : ReferenceEquals(value, first) ? "first"
            : ReferenceEquals(value, second) ? "second" : ReferenceEquals(value, shared) ? "shared"
            : value.GetType() == pair ? "pair:" + Describe(pair.GetField("Count").GetValue(value)) + ":" + Describe(pair.GetField("Tag").GetValue(value))
            : value.GetType().FullName + ":" + value;
        var foreign = library.GetType("StaticFieldLibrary.ForeignSlots`1") ?? throw new Exception("Owned foreign generic storage missing.");
        var globals = library.GetType("StaticFieldLibrary.Globals") ?? throw new Exception("Owned foreign globals missing.");
        var valueSlots = fixture.GetType("StaticFieldCases.LocalValueSlots`1") ?? throw new Exception("Owned value slots missing.");
        var foreignValueSlots = library.GetType("StaticFieldLibrary.ValueSlots`1") ?? throw new Exception("Owned foreign value slots missing.");
        var mixedSlots = fixture.GetType("StaticFieldCases.MixedSlots`2") ?? throw new Exception("Owned mixed storage missing.");
        var threadSlots = fixture.GetType("StaticFieldCases.ThreadSlots`1") ?? throw new Exception("Owned thread slots missing.");
        var fields = types.SelectMany(t => slots.MakeGenericType(t).GetFields(BindingFlags.Public | BindingFlags.Static))
            .Concat(types.SelectMany(t => foreign.MakeGenericType(t).GetFields(BindingFlags.Public | BindingFlags.Static)))
            .Concat(globals.GetFields(BindingFlags.Public | BindingFlags.Static))
            .Concat(types.SelectMany(t => valueSlots.MakeGenericType(t).GetFields(BindingFlags.Public | BindingFlags.Static)))
            .Concat(types.SelectMany(t => foreignValueSlots.MakeGenericType(t).GetFields(BindingFlags.Public | BindingFlags.Static)))
            .Concat(types.SelectMany(t => types.SelectMany(u => mixedSlots.MakeGenericType(t, u).GetFields(BindingFlags.Public | BindingFlags.Static))))
            .Concat(types.SelectMany(t => threadSlots.MakeGenericType(t).GetFields(BindingFlags.Public | BindingFlags.Static)))
            .OrderBy(f => f.DeclaringType.ToString()).ThenBy(f => f.Name).ToArray();
        int comparisons = 0, selections = 0;
        foreach (var row in threadsOnly ? Array.Empty<JsonElement>() : plan.RootElement.EnumerateArray().ToArray())
        {
            var original = (MethodInfo)fixture.ManifestModule.ResolveMethod(row.GetProperty("token").GetInt32());
            if (row.GetProperty("type_argument").ValueKind != JsonValueKind.Null)
                original = original.DeclaringType.MakeGenericType(Type.GetType(row.GetProperty("type_argument").GetString(), true))
                    .GetMethods().Single(m => m.MetadataToken == original.MetadataToken);
            if (original.IsGenericMethodDefinition) original = original.MakeGenericMethod(Type.GetType(row.GetProperty("argument").GetString(), true));
            if (original.Name.StartsWith("Init", StringComparison.Ordinal)) continue; // First-touch cases run in separate fresh processes.
            var lowered = generated.GetType(row.GetProperty("carrier").GetString() + ".Recovered")?.GetMethod(original.Name)
                ?? throw new Exception("Recovered static field method missing.");
            if (original.ReturnType != lowered.ReturnType || !original.GetParameters().Select(p => p.ParameterType)
                .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType))) throw new Exception("Static field ABI differs.");
            selections++;
            for (int initial = 0; initial < 3; initial++)
            for (int variant = 0; variant < 3; variant++)
            {
                object[] arguments = original.GetParameters().Select(p => Sample(p.ParameterType, variant)).ToArray();
                (string value, string error, string state) Execute(MethodInfo method)
                {
                    for (int i = 0; i < fields.Length; i++)
                        fields[i].SetValue(null, fields[i].FieldType == typeof(int) ? (object)unchecked(initial * 100003 + i * 97) : Sample(fields[i].FieldType, (initial + i) % 3));
                    object result = null; Exception error = null;
                    try { result = ManagedInvoker(method)(arguments); } catch (Exception e) { error = e; }
                    return (Describe(result), error?.GetType().FullName,
                        string.Join("|", fields.Select(f => Describe(f.GetValue(null)))));
                }
                var before = Execute(original); var after = Execute(lowered);
                if (before != after) throw new Exception("Recovered static field differs: " + original + " expected=" + before + " actual=" + after);
                comparisons++;
            }
        }
        int threadComparisons = 0;
        foreach (var row in plan.RootElement.EnumerateArray()) {
            var definition = (MethodInfo)fixture.ManifestModule.ResolveMethod(row.GetProperty("token").GetInt32());
            if (definition.Name != "ReadThread") continue;
            Type argument = Type.GetType(row.GetProperty("argument").GetString(), true);
            var originalRead = definition.MakeGenericMethod(argument);
            var originalWrite = definition.DeclaringType.GetMethod("WriteThread").MakeGenericMethod(argument);
            var read = generated.GetType(row.GetProperty("carrier").GetString() + ".Recovered").GetMethod("ReadThread");
            var writeRow = plan.RootElement.EnumerateArray().Single(r =>
                ((MethodInfo)fixture.ManifestModule.ResolveMethod(r.GetProperty("token").GetInt32())).Name == "WriteThread"
                && r.GetProperty("argument").GetString() == row.GetProperty("argument").GetString());
            var write = generated.GetType(writeRow.GetProperty("carrier").GetString() + ".Recovered").GetMethod("WriteThread");
            var invokeOriginalRead = ManagedInvoker(originalRead);
            var invokeOriginalWrite = ManagedInvoker(originalWrite);
            var invokeRead = ManagedInvoker(read);
            var invokeWrite = ManagedInvoker(write);
            var errors = new System.Collections.Concurrent.ConcurrentQueue<Exception>();
            using var barrier = new System.Threading.Barrier(2);
            var threads = Enumerable.Range(0, 2).Select(index => new System.Threading.Thread(() => {
                int value = 1001 + index * 113;
                Exception error = null;
                try { invokeOriginalWrite(new object[] { value }); } catch (Exception e) { error = e; }
                barrier.SignalAndWait();
                try {
                    if (error != null || (int)invokeOriginalRead(Array.Empty<object>()) != value
                        || (int)invokeRead(Array.Empty<object>()) != value)
                        throw new Exception("Recovered thread static differs: read");
                    if ((int)invokeWrite(new object[] { value + 7 }) != value + 7)
                        throw new Exception("Recovered thread static differs: write");
                } catch (Exception e) { error = e; }
                barrier.SignalAndWait();
                try {
                    if ((int)invokeOriginalRead(Array.Empty<object>()) != value + 7)
                        throw new Exception("Recovered thread static differs: shared storage");
                } catch (Exception e) { error = e; }
                if (error != null) errors.Enqueue(error);
            })).ToArray();
            foreach (var thread in threads) thread.Start();
            foreach (var thread in threads) thread.Join();
            if (!errors.IsEmpty) throw new Exception("Recovered thread static differs.", errors.First());
            threadComparisons += 4;
        }
        Console.WriteLine("Thread static behavior: " + threadComparisons + " barrier-synchronized comparisons.");
        Console.WriteLine("Static field behavior: " + comparisons + " comparisons, " + selections
            + " closed selections; closed storage, typed values, identities and effects match.");
    }
}
