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
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;

internal static class FrozenNamingMethods
{
    const BindingFlags Members = BindingFlags.Public | BindingFlags.NonPublic |
        BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
    static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException("Invalid gate input: " + message);
    }
    static void Print(object value) => Console.WriteLine(JsonSerializer.Serialize(value));
    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";
    static string Text(JsonElement value, string key) => value.GetProperty(key).GetString();
    static int Id(JsonElement value, string key) => Convert.ToInt32(Text(value, key)[2..], 16);

    // Cataloging never loads or executes an input assembly.
    static void Catalog(string path, string rootToken)
    {
        using var pe = new PEReader(File.OpenRead(path));
        var md = pe.GetMetadataReader();
        string[] TypePath(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parent = type.GetDeclaringType();
            return (parent.IsNil ? Array.Empty<string>() : TypePath(parent)).Append(md.GetString(type.Name)).ToArray();
        }
        string Namespace(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            return type.GetDeclaringType().IsNil ? md.GetString(type.Namespace) : Namespace(type.GetDeclaringType());
        }
        object Method(MethodDefinitionHandle handle)
        {
            var method = md.GetMethodDefinition(handle);
            var body = method.RelativeVirtualAddress == 0 ? null : pe.GetMethodBody(method.RelativeVirtualAddress);
            return new { token = Token(handle), name = md.GetString(method.Name), flags = (int)method.Attributes,
                implFlags = (int)method.ImplAttributes, signature = Convert.ToHexString(md.GetBlobBytes(method.Signature)),
                il = Convert.ToHexString(body?.GetILBytes() ?? Array.Empty<byte>()), eh = body?.ExceptionRegions.Length ?? 0 };
        }
        var root = md.TypeDefinitions.Single(h => Token(h) == rootToken);
        bool Owned(TypeDefinitionHandle handle)
        {
            if (handle == root) return true;
            var parent = md.GetTypeDefinition(handle).GetDeclaringType();
            return !parent.IsNil && Owned(parent);
        }
        bool Generic(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            return type.GetGenericParameters().Count != 0 || !type.GetDeclaringType().IsNil && Generic(type.GetDeclaringType());
        }
        Print(new { mvid = md.GetGuid(md.GetModuleDefinition().Mvid).ToString(),
            moduleInitializers = md.TypeDefinitions.Where(h => md.GetString(md.GetTypeDefinition(h).Name) == "<Module>")
                .SelectMany(h => md.GetTypeDefinition(h).GetMethods())
                .Where(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor").Select(Method).ToArray(),
            types = md.TypeDefinitions.Where(Owned).Select(handle => {
                var type = md.GetTypeDefinition(handle);
                var methods = type.GetMethods().ToArray();
                return new { token = Token(handle), name = Namespace(handle) + "." + string.Join("/", TypePath(handle)),
                    path = TypePath(handle), @namespace = Namespace(handle), flags = (int)type.Attributes,
                    baseToken = Token(type.BaseType), generic = Generic(handle),
                    finalizer = methods.Any(h => md.GetString(md.GetMethodDefinition(h).Name) == "Finalize"),
                    initializers = methods.Where(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor").Select(Method).ToArray(),
                    methods = methods.Select(Method).ToArray(), fields = type.GetFields().Select(h => {
                        var field = md.GetFieldDefinition(h);
                        return new { token = Token(h), name = md.GetString(field.Name), flags = (int)field.Attributes,
                            signature = Convert.ToHexString(md.GetBlobBytes(field.Signature)) };
                    }).ToArray() };
            }).ToArray() });
    }

    sealed class CorpusContext : AssemblyLoadContext
    {
        readonly string corpus;
        public CorpusContext(string corpus) : base(isCollectible: true) { this.corpus = corpus; }
        protected override Assembly Load(AssemblyName name)
        {
            foreach (string directory in new[] { "assemblies", "runtime-deps" })
            {
                string file = Path.Combine(corpus, directory, name.Name + ".dll");
                if (File.Exists(file)) return LoadFromAssemblyPath(file);
            }
            return null;
        }
    }

    static void Safe(Type type)
    {
        Require(type != null && type.BaseType == typeof(object) && type.TypeInitializer == null &&
            !type.ContainsGenericParameters && !type.GetMethods(Members).Any(m => m.Name == "Finalize"),
            "Object-based owner without initialization, generics or finalizer: " + type);
    }

    static Func<object, object, object> Direct(MethodInfo method)
    {
        var bridge = new DynamicMethod("direct_selected_method", typeof(object), new[] { typeof(object), typeof(object) },
            typeof(FrozenNamingMethods).Module, true);
        var il = bridge.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Castclass, method.DeclaringType);
        if (method.GetParameters().Length == 1)
        {
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(OpCodes.Castclass, method.GetParameters()[0].ParameterType);
        }
        // Direct call, including for private virtual interface implementations:
        // no added callvirt receiver check and no original constructor/factory.
        il.Emit(OpCodes.Call, method);
        if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
        else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
        il.Emit(OpCodes.Ret);
        return bridge.CreateDelegate<Func<object, object, object>>();
    }

    static readonly Dictionary<short, OpCode> Opcodes = typeof(OpCodes).GetFields()
        .Where(f => f.FieldType == typeof(OpCode)).Select(f => (OpCode)f.GetValue(null)).ToDictionary(o => o.Value);

    // Bounded compiler IL, not an interpreter. All field operands must bind to
    // the independently inventoried field map; calls/proxies are never permitted.
    static void NoCalls(MethodInfo method, IEnumerable<FieldInfo> fields)
    {
        var body = method.GetMethodBody();
        Require(body != null && body.ExceptionHandlingClauses.Count == 0, "ordinary method body without EH");
        byte[] bytes = body.GetILAsByteArray();
        var allowed = new HashSet<string>(new[] { "nop", "ret", "pop", "ldarg.0", "ldarg.1", "ldnull", "ldfld", "stfld",
            "ldc.i4.m1", "ldc.i4.0", "ldc.i4.1", "ldc.i4.2", "ldc.i4.3", "ldc.i4.4", "ldc.i4.5", "ldc.i4.6",
            "ldc.i4.7", "ldc.i4.8", "ldc.i4.s", "ldc.i4", "ceq", "cgt.un", "ldloc.0", "ldloc.1", "ldloc.2",
            "ldloc.3", "stloc.0", "stloc.1", "stloc.2", "stloc.3", "br.s", "brfalse.s", "brtrue.s" });
        for (int offset = 0; offset < bytes.Length;)
        {
            short code = bytes[offset++];
            if (code == 0xFE) code = (short)(0xFE00 | bytes[offset++]);
            Require(Opcodes.TryGetValue(code, out var op) && allowed.Contains(op.Name), "call-free scalar IL: " + op.Name);
            int size = op.OperandType switch {
                OperandType.InlineNone => 0,
                OperandType.ShortInlineI or OperandType.ShortInlineBrTarget => 1,
                OperandType.InlineI or OperandType.InlineField => 4,
                _ => throw new InvalidOperationException("Unexpected scalar operand: " + op)
            };
            Require(offset + size <= bytes.Length, "complete scalar operand");
            if (op.OperandType == OperandType.InlineField)
            {
                var field = method.Module.ResolveField(BitConverter.ToInt32(bytes, offset));
                Require(fields.Contains(field) && !field.IsStatic, "only observed instance fields; no proxy field");
            }
            offset += size;
        }
    }

    sealed class Side
    {
        public string Name;
        public Type Type;
        public FieldInfo[] Fields;
        public Func<object, object, object> Invoke;
        public object Fresh() { Safe(Type); return RuntimeHelpers.GetUninitializedObject(Type); }
    }

    sealed class Mismatch : Exception
    {
        public string Kind, Side, Context;
        public Mismatch(string kind, string side, string context) { Kind = kind; Side = side; Context = context; }
    }
    static void Check(bool condition, string kind, Side side, string context)
    {
        if (!condition) throw new Mismatch(kind, side.Name, context);
    }
    static object Call(Side side, object receiver, string expectedException, string context)
    {
        object result = null;
        Exception failure = null;
        try { result = side.Invoke(receiver, null); }
        catch (Exception exception) { failure = exception; }
        Check(failure?.GetType().FullName == expectedException, "exception", side, context);
        return result;
    }
    sealed class Snapshot
    {
        public readonly string[] Unobserved;
        public readonly string[] Observed;
        readonly Side side;
        readonly object receiver;
        readonly FieldInfo[] fields;
        readonly object[] values;
        public Snapshot(Side side, object receiver, params FieldInfo[] excluded)
        {
            this.side = side;
            this.receiver = receiver;
            var all = receiver.GetType().GetFields(Members).Where(f => !f.IsStatic && !excluded.Contains(f)).ToArray();
            fields = all.Where(f => !f.FieldType.IsValueType || f.FieldType.IsPrimitive).ToArray();
            Unobserved = all.Except(fields).Select(f => f.Name).ToArray();
            Observed = fields.Select(f => f.Name).ToArray();
            values = fields.Select(f => f.GetValue(receiver)).ToArray();
        }
        public void Verify(string context)
        {
            for (int i = 0; i < fields.Length; i++)
            {
                object actual = fields[i].GetValue(receiver);
                Check(fields[i].FieldType.IsPrimitive ? Equals(actual, values[i]) : ReferenceEquals(actual, values[i]),
                    "preserved-state", side, context + ":" + fields[i].Name);
            }
        }
    }

    static string Alias(string raw) => raw.Length > 0 && (raw[0] == '_' || char.IsAsciiLetter(raw[0])) &&
        raw.All(c => c == '_' || char.IsAsciiLetterOrDigit(c)) &&
        !raw.StartsWith("__name_", StringComparison.Ordinal) && !raw.StartsWith("__cil_", StringComparison.Ordinal) &&
        !raw.StartsWith("__il_", StringComparison.Ordinal) ? raw : "__name_" + Convert.ToHexString(System.Text.Encoding.UTF8.GetBytes(raw));

    static void Execute(string corpus, string evidencePath, string recoveredPath)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(evidencePath));
        var entry = document.RootElement;
        var methodRecord = entry.GetProperty("method");
        var owner = entry.GetProperty("owner");
        string token = Text(methodRecord, "token"), family = Text(entry, "family");
        string assemblyPath = Path.Combine(corpus, "assemblies", Text(entry, "assembly"));
        using var pe = new PEReader(File.OpenRead(assemblyPath));
        var md = pe.GetMetadataReader();
        var definition = md.GetMethodDefinition((MethodDefinitionHandle)MetadataTokens.EntityHandle(Id(methodRecord, "token")));
        Require(Convert.ToHexString(md.GetBlobBytes(definition.Signature)) == Text(methodRecord, "signature") &&
            Token(definition.GetDeclaringType()) == Text(owner, "token"), "independent method signature and ownership");
        var context = new CorpusContext(corpus);
        var original = context.LoadFromAssemblyPath(assemblyPath);
        Require(original.ManifestModule.ModuleVersionId.ToString() == Text(entry, "mvid") &&
            original.ManifestModule.GetType("<Module>")?.TypeInitializer == null, "loaded MVID/module initialization");
        var leftMethod = original.ManifestModule.ResolveMethod(Id(methodRecord, "token")) as MethodInfo;
        Require(leftMethod != null && leftMethod.Name == Text(methodRecord, "name") && !leftMethod.IsStatic &&
            !leftMethod.IsGenericMethod && leftMethod.DeclaringType.MetadataToken == Id(owner, "token") &&
            leftMethod.DeclaringType.FullName.Replace('+', '/') == Text(owner, "name") &&
            (int)leftMethod.Attributes == methodRecord.GetProperty("flags").GetInt32() &&
            (int)leftMethod.GetMethodImplementationFlags() == methodRecord.GetProperty("implFlags").GetInt32(), "loaded method identity/flags");
        Require(Convert.ToHexString(leftMethod.GetMethodBody().GetILAsByteArray()) == Text(methodRecord, "il") &&
            leftMethod.GetMethodBody().ExceptionHandlingClauses.Count == 0, "loaded original/raw PE IL agreement");
        var parameters = leftMethod.GetParameters();
        Require(family == "empty-return" ? parameters.Length == 1 && parameters[0].ParameterType.FullName == "OpenRA.Actor" &&
            parameters[0].ParameterType.Assembly.GetName().Name == "OpenRA.Game" : parameters.Length == 0, "actual original argument ABI");
        Type expectedReturn = family == "string-getter" ? typeof(string) : family == "negate-visible" ? typeof(bool) : typeof(void);
        Require(leftMethod.ReturnType == expectedReturn, "actual original result ABI");
        var recovered = new AssemblyLoadContext("recovered", true).LoadFromAssemblyPath(recoveredPath);
        Require(recovered.GetReferencedAssemblies().All(a => a.Name == "System.Runtime" || a.Name == "System.Private.CoreLib"),
            "only framework compiler references; no original-selected proxy");
        string ns = Text(owner, "namespace");
        string ShellType(JsonElement path) => ns + "." + string.Join("+", path.EnumerateArray().Select(p => Alias(p.GetString())));
        string[] ownerPath = owner.GetProperty("path").EnumerateArray().Select(p => p.GetString()).ToArray();
        var expectedTypes = Enumerable.Range(1, ownerPath.Length)
            .Select(n => ns + "." + string.Join("+", ownerPath.Take(n).Select(Alias))).ToArray();
        Require(recovered.GetTypes().Select(t => t.FullName).OrderBy(s => s).SequenceEqual(expectedTypes.OrderBy(s => s)),
            "only explicitly declared shadow owner/context types");
        foreach (var type in recovered.GetTypes()) Safe(type);
        var rightType = recovered.GetType(Text(entry, "recoveredType"), throwOnError: true);
        var rightMethod = rightType.GetMethod(Text(entry.GetProperty("extraction"), "methodName"), Members);
        Require(rightMethod != null && !rightMethod.IsStatic && !rightMethod.IsVirtual && rightMethod.IsPublic &&
            rightMethod.ReturnType == expectedReturn && rightMethod.GetParameters().Select(p => p.ParameterType)
                .SequenceEqual(family == "empty-return" ? new[] { typeof(object) } : Type.EmptyTypes), "explicit test shell ABI");
        var leftFields = new List<FieldInfo>();
        var rightFields = new List<FieldInfo>();
        var fieldReports = new List<object>();
        foreach (var fieldRecord in entry.GetProperty("fields").EnumerateArray())
        {
            var left = original.ManifestModule.ResolveField(Id(fieldRecord, "token"));
            var right = recovered.GetType(ShellType(fieldRecord.GetProperty("ownerPath")), true)
                .GetField(Text(fieldRecord, "shellName"), Members);
            Require(left != null && right != null && !left.IsStatic && !right.IsStatic && left.Name == Text(fieldRecord, "name") &&
                left.DeclaringType.MetadataToken == Id(fieldRecord, "ownerToken"), "scoped field correspondence");
            var rawField = md.GetFieldDefinition((FieldDefinitionHandle)MetadataTokens.EntityHandle(Id(fieldRecord, "token")));
            Require(Convert.ToHexString(md.GetBlobBytes(rawField.Signature)) == Text(fieldRecord, "signature") &&
                (int)left.Attributes == fieldRecord.GetProperty("flags").GetInt32(), "independent field signature/flags");
            string shellType = Text(fieldRecord, "shellType");
            if (shellType.StartsWith("global::", StringComparison.Ordinal))
            {
                Require(left.FieldType.FullName == shellType[8..] && right.FieldType.FullName == shellType[8..] &&
                    left.FieldType.Assembly == original && right.FieldType.Assembly == recovered, "independent shadow referent ABI");
                Safe(left.FieldType);
                Safe(right.FieldType);
            }
            else
            {
                Type expected = shellType == "string" ? typeof(string) : shellType == "int" ? typeof(int) : typeof(bool);
                Require(left.FieldType == expected && right.FieldType == expected, "primitive/string field ABI");
            }
            leftFields.Add(left);
            rightFields.Add(right);
            fieldReports.Add(new { token = Text(fieldRecord, "token"), originalType = left.FieldType.AssemblyQualifiedName,
                originalFlags = (int)left.Attributes, recoveredType = right.FieldType.AssemblyQualifiedName });
        }
        Require(recovered.GetTypes().Sum(t => t.GetFields(Members).Length) == rightFields.Count, "only explicitly bounded fields");
        var leftSide = new Side { Name = "original", Type = leftMethod.DeclaringType, Fields = leftFields.ToArray() };
        var rightSide = new Side { Name = "recovered", Type = rightType, Fields = rightFields.ToArray() };
        Safe(leftSide.Type);
        NoCalls(leftMethod, leftSide.Fields);
        NoCalls(rightMethod, rightSide.Fields);
        leftSide.Invoke = Direct(leftMethod);
        rightSide.Invoke = Direct(rightMethod);
        int pairs = 0, nullThisPairs = 0, nullReferentPairs = 0;
        var unobserved = new HashSet<string>();
        var snapshotFields = new HashSet<string>();
        var sampled = new List<string>();
        Snapshot Remember(Side side, object receiver, params FieldInfo[] excluded)
        {
            var snapshot = new Snapshot(side, receiver, excluded);
            foreach (var name in snapshot.Unobserved) unobserved.Add(side.Name + ":" + receiver.GetType().FullName + ":" + name);
            foreach (var name in snapshot.Observed) snapshotFields.Add(side.Name + ":" + receiver.GetType().FullName + ":" + name);
            return snapshot;
        }
        void NullReceivers(string expected)
        {
            for (int repeat = 0; repeat < 3; repeat++)
            {
                Call(leftSide, null, expected, "null-this");
                Call(rightSide, null, expected, "null-this");
                pairs++;
                nullThisPairs++;
            }
        }
        try
        {
            if (family == "string-getter")
            {
                string[] samples = { null, "", "alpha", "alpha", "\0", "\uD7FF\uD800\uDBFF\uDC00\uDFFF\uE000\uFFFF",
                    "x\uD800", "\uDFFFy", "\uD83D\uDE00" };
                foreach (string sample in samples)
                {
                    string leftValue = sample == null ? null : new string(sample.ToCharArray());
                    string rightValue = sample == null ? null : new string(sample.ToCharArray());
                    if (!string.IsNullOrEmpty(sample)) Require(!ReferenceEquals(leftValue, rightValue), "independently prepared string identities");
                    sampled.Add(sample == null ? "null" : string.Join(" ", sample.Select(c => ((int)c).ToString("X4"))));
                    object leftReceiver = leftSide.Fresh(), rightReceiver = rightSide.Fresh();
                    leftSide.Fields[0].SetValue(leftReceiver, leftValue);
                    rightSide.Fields[0].SetValue(rightReceiver, rightValue);
                    var beforeLeft = Remember(leftSide, leftReceiver);
                    var beforeRight = Remember(rightSide, rightReceiver);
                    for (int repeat = 0; repeat < 3; repeat++)
                    {
                        Check(ReferenceEquals(Call(leftSide, leftReceiver, null, "string"), leftValue), "return-identity", leftSide, "string");
                        beforeLeft.Verify("string");
                        Check(ReferenceEquals(Call(rightSide, rightReceiver, null, "string"), rightValue), "return-identity", rightSide, "string");
                        beforeRight.Verify("string");
                        pairs++;
                    }
                }
                NullReceivers(typeof(NullReferenceException).FullName);
            }
            else if (family == "dispose-store" || family == "clear-visible" || family == "empty-return")
            {
                object[] seeds = family == "dispose-store" ? new object[] { int.MinValue, int.MinValue + 1, -3, -2, -1, 0, 1, int.MaxValue - 1, int.MaxValue }
                    : new object[] { false, true };
                foreach (object seed in seeds)
                {
                    object leftReceiver = leftSide.Fresh(), rightReceiver = rightSide.Fresh();
                    leftSide.Fields[0].SetValue(leftReceiver, seed);
                    rightSide.Fields[0].SetValue(rightReceiver, seed);
                    var beforeLeft = Remember(leftSide, leftReceiver, leftSide.Fields[0]);
                    var beforeRight = Remember(rightSide, rightReceiver, rightSide.Fields[0]);
                    object expected = family == "dispose-store" ? -2 : family == "clear-visible" ? false : seed;
                    for (int repeat = 0; repeat < 3; repeat++)
                    {
                        Call(leftSide, leftReceiver, null, "store");
                        Check(Equals(leftSide.Fields[0].GetValue(leftReceiver), expected), family == "empty-return" ? "preserved-state" : "store-state", leftSide, "store");
                        beforeLeft.Verify("store");
                        Call(rightSide, rightReceiver, null, "store");
                        Check(Equals(rightSide.Fields[0].GetValue(rightReceiver), expected), family == "empty-return" ? "preserved-state" : "store-state", rightSide, "store");
                        beforeRight.Verify("store");
                        pairs++;
                    }
                }
                NullReceivers(family == "empty-return" ? null : typeof(NullReferenceException).FullName);
            }
            else
            {
                Require(family == "negate-visible", "known behavior family");
                foreach (bool seed in new[] { false, true })
                {
                    object leftReceiver = leftSide.Fresh(), rightReceiver = rightSide.Fresh();
                    object leftReferent = RuntimeHelpers.GetUninitializedObject(leftSide.Fields[0].FieldType);
                    object rightReferent = RuntimeHelpers.GetUninitializedObject(rightSide.Fields[0].FieldType);
                    Require(!ReferenceEquals(leftReferent, rightReferent), "independently allocated shadow referents");
                    leftSide.Fields[1].SetValue(leftReferent, seed);
                    rightSide.Fields[1].SetValue(rightReferent, seed);
                    leftSide.Fields[0].SetValue(leftReceiver, leftReferent);
                    rightSide.Fields[0].SetValue(rightReceiver, rightReferent);
                    var snapshots = new[] { Remember(leftSide, leftReceiver), Remember(rightSide, rightReceiver),
                        Remember(leftSide, leftReferent), Remember(rightSide, rightReferent) };
                    for (int repeat = 0; repeat < 3; repeat++)
                    {
                        Check(Equals(Call(leftSide, leftReceiver, null, "predicate"), !seed), "return-value", leftSide, "predicate");
                        Check(Equals(Call(rightSide, rightReceiver, null, "predicate"), !seed), "return-value", rightSide, "predicate");
                        foreach (var snapshot in snapshots) snapshot.Verify("predicate");
                        pairs++;
                    }
                }
                object nullLeft = leftSide.Fresh(), nullRight = rightSide.Fresh();
                var nullSnapshots = new[] { Remember(leftSide, nullLeft), Remember(rightSide, nullRight) };
                for (int repeat = 0; repeat < 3; repeat++)
                {
                    Call(leftSide, nullLeft, typeof(NullReferenceException).FullName, "null-referent");
                    Call(rightSide, nullRight, typeof(NullReferenceException).FullName, "null-referent");
                    foreach (var snapshot in nullSnapshots) snapshot.Verify("null-referent");
                    pairs++;
                    nullReferentPairs++;
                }
                NullReceivers(typeof(NullReferenceException).FullName);
            }
            Print(new { status = "passed", token, family, pairs, nullThisPairs, nullReferentPairs,
                nonnullPairs = pairs - nullThisPairs - nullReferentPairs, repeats = 3, noCalls = true, noOriginalProxy = true,
                recoveredIl = Convert.ToHexString(rightMethod.GetMethodBody().GetILAsByteArray()),
                originalFlags = (int)leftMethod.Attributes, originalReturnType = leftMethod.ReturnType.AssemblyQualifiedName,
                originalParameterTypes = parameters.Select(p => p.ParameterType.AssemblyQualifiedName).ToArray(),
                recoveredParameterTypes = rightMethod.GetParameters().Select(p => p.ParameterType.AssemblyQualifiedName).ToArray(),
                fieldTypes = fieldReports, snapshotFields = snapshotFields.OrderBy(s => s).ToArray(),
                unobservedFields = unobserved.OrderBy(s => s).ToArray(), stringSamplesUtf16 = sampled });
        }
        catch (Mismatch failure)
        {
            Print(new { status = "behavior-mismatch", token, family, kind = failure.Kind, side = failure.Side,
                context = failure.Context, completedPairs = pairs, noCalls = true, noOriginalProxy = true,
                recoveredIl = Convert.ToHexString(rightMethod.GetMethodBody().GetILAsByteArray()) });
            Environment.ExitCode = 42;
        }
    }

    static void Main(string[] args)
    {
        Require(args.Length == 3 && args[0] == "catalog" || args.Length == 4 && args[0] == "execute", "command arguments");
        if (args[0] == "catalog") Catalog(args[1], args[2]);
        else Execute(args[1], args[2], args[3]);
    }
}
