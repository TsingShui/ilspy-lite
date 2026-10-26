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

// Original initialization is an oracle-only precondition, never a recovered leaf.
internal static class FrozenNamingSingletons
{
    const BindingFlags Members = BindingFlags.Public | BindingFlags.NonPublic |
        BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
    const string MutationMessage = "frozen-singleton-ctor-mutation";
    static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException("Invalid gate input: " + message);
    }
    static void Print(object value) => Console.WriteLine(JsonSerializer.Serialize(value));
    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";
    static string Text(JsonElement value, string key) => value.GetProperty(key).GetString();
    static int Id(JsonElement value, string key) => Convert.ToInt32(Text(value, key)[2..], 16);

    // PEReader never loads the corpus. Only this exact tiny protocol is admitted.
    static JsonElement Inventory(string path, string ownerToken)
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
        bool CoreObject(EntityHandle handle)
        {
            if (handle.Kind != HandleKind.TypeReference) return false;
            var type = md.GetTypeReference((TypeReferenceHandle)handle);
            if (md.GetString(type.Name) != "Object" || md.GetString(type.Namespace) != "System" ||
                type.ResolutionScope.Kind != HandleKind.AssemblyReference) return false;
            var scope = md.GetAssemblyReference((AssemblyReferenceHandle)type.ResolutionScope);
            return md.GetString(scope.Name) == "System.Runtime" && scope.Version == new Version(6, 0, 0, 0) &&
                scope.Flags == 0 && md.GetString(scope.Culture) == "" &&
                Convert.ToHexString(md.GetBlobBytes(scope.PublicKeyOrToken)) == "B03F5F7F11D50A3A";
        }
        byte[] Bytes(MethodDefinitionHandle handle)
        {
            var method = md.GetMethodDefinition(handle);
            Require(method.RelativeVirtualAddress != 0, "ordinary PE body");
            var body = pe.GetMethodBody(method.RelativeVirtualAddress);
            Require(body.ExceptionRegions.Length == 0, "no exception regions");
            return body.GetILBytes();
        }
        object Method(MethodDefinitionHandle handle)
        {
            var method = md.GetMethodDefinition(handle);
            return new { token = Token(handle), name = md.GetString(method.Name), flags = (int)method.Attributes,
                implFlags = (int)method.ImplAttributes, signature = Convert.ToHexString(md.GetBlobBytes(method.Signature)),
                il = Convert.ToHexString(Bytes(handle)), eh = 0 };
        }
        var moduleInitializers = md.TypeDefinitions.Where(h => md.GetString(md.GetTypeDefinition(h).Name) == "<Module>")
            .SelectMany(h => md.GetTypeDefinition(h).GetMethods())
            .Where(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor").ToArray();
        Require(moduleInitializers.Length == 0, "no module initializer");
        var owner = md.TypeDefinitions.Single(h => Token(h) == ownerToken);
        var definition = md.GetTypeDefinition(owner);
        var methods = definition.GetMethods().ToArray();
        Require(CoreObject(definition.BaseType) && definition.GetGenericParameters().Count == 0 &&
            !methods.Any(h => md.GetString(md.GetMethodDefinition(h).Name) == "Finalize"), "Object-only nongeneric owner");
        var ctor = methods.Single(h => md.GetString(md.GetMethodDefinition(h).Name) == ".ctor");
        var ctorDef = md.GetMethodDefinition(ctor);
        byte[] ctorBytes = Bytes(ctor);
        Require(Convert.ToHexString(md.GetBlobBytes(ctorDef.Signature)) == "200001" &&
            ctorBytes.Length == 7 && ctorBytes[0] == 0x02 && ctorBytes[1] == 0x28 && ctorBytes[6] == 0x2A,
            "exact field-free parameterless direct-call constructor");
        var callHandle = MetadataTokens.EntityHandle(BitConverter.ToInt32(ctorBytes, 2));
        Require(callHandle.Kind == HandleKind.MemberReference, "core constructor MemberRef");
        var call = md.GetMemberReference((MemberReferenceHandle)callHandle);
        Require(md.GetString(call.Name) == ".ctor" && CoreObject(call.Parent) &&
            Convert.ToHexString(md.GetBlobBytes(call.Signature)) == "200001", "exact core Object constructor identity");
        var initializer = methods.Single(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor");
        var initDef = md.GetMethodDefinition(initializer);
        byte[] initBytes = Bytes(initializer);
        Require(Convert.ToHexString(md.GetBlobBytes(initDef.Signature)) == "000001" &&
            initDef.Attributes.HasFlag(MethodAttributes.Static) && initBytes.Length == 11 && initBytes[0] == 0x73 &&
            initBytes[5] == 0x80 && initBytes[10] == 0x2A && BitConverter.ToInt32(initBytes, 1) == MetadataTokens.GetToken(ctor),
            "initializer is only newobj own Object-only ctor; stsfld; ret");
        int fieldToken = BitConverter.ToInt32(initBytes, 6);
        var singleton = definition.GetFields().Single(h => MetadataTokens.GetToken(h) == fieldToken);
        var fieldDef = md.GetFieldDefinition(singleton);
        var signature = md.GetBlobReader(fieldDef.Signature);
        Require(signature.ReadByte() == 0x06 && signature.ReadByte() == 0x12 && signature.ReadTypeHandle() == owner &&
            signature.RemainingBytes == 0 && fieldDef.Attributes.HasFlag(FieldAttributes.Static | FieldAttributes.InitOnly),
            "singleton is readonly static field of own owner");
        Require(definition.GetFields().All(h => md.GetFieldDefinition(h).Attributes.HasFlag(FieldAttributes.Static)),
            "no original instance fields; dirty receivers mean constructor-warmed only");
        return JsonSerializer.SerializeToElement(new { mvid = md.GetGuid(md.GetModuleDefinition().Mvid).ToString(),
            moduleInitializers = Array.Empty<object>(), owner = new { token = Token(owner),
                name = Namespace(owner) + "." + string.Join("/", TypePath(owner)), path = TypePath(owner),
                @namespace = Namespace(owner), flags = (int)definition.Attributes, objectBase = true,
                singleton = Token(singleton), constructor = Token(ctor), initializer = Token(initializer),
                fields = definition.GetFields().Select(h => {
                    var field = md.GetFieldDefinition(h);
                    return new { token = Token(h), name = md.GetString(field.Name), flags = (int)field.Attributes,
                        signature = Convert.ToHexString(md.GetBlobBytes(field.Signature)) };
                }).ToArray(), methods = methods.Select(Method).ToArray() } });
    }

    sealed class CorpusContext : AssemblyLoadContext
    {
        readonly string corpus;
        public CorpusContext(string corpus) : base(isCollectible: true) { this.corpus = corpus; }
        protected override Assembly Load(AssemblyName name)
        {
            foreach (string directory in new[] { "assemblies", "runtime-deps" })
            {
                string path = Path.Combine(corpus, directory, name.Name + ".dll");
                if (File.Exists(path)) return LoadFromAssemblyPath(path);
            }
            return null;
        }
    }
    static byte[] Body(MethodBase method)
    {
        var body = method.GetMethodBody();
        Require(body != null && body.ExceptionHandlingClauses.Count == 0, "ordinary runtime body without EH");
        return body.GetILAsByteArray();
    }
    static bool CoreTarget(Module module, int token) => module.ResolveMethod(token) is ConstructorInfo ctor &&
        ctor.DeclaringType == typeof(object) && !ctor.IsStatic && ctor.GetParameters().Length == 0;
    static void VerifyConstructor(ConstructorInfo ctor, bool mutation)
    {
        byte[] bytes = Body(ctor);
        Require(bytes.Length == (mutation ? 17 : 7) && bytes[0] == 0x02 && bytes[1] == 0x28 &&
            CoreTarget(ctor.Module, BitConverter.ToInt32(bytes, 2)), "exact core Object direct call, no proxy");
        if (!mutation) Require(bytes[6] == 0x2A, "constructor ret");
        else Require(bytes[6] == 0x72 && bytes[11] == 0x73 && bytes[16] == 0x7A &&
            ctor.Module.ResolveString(BitConverter.ToInt32(bytes, 7)) == MutationMessage &&
            ctor.Module.ResolveMethod(BitConverter.ToInt32(bytes, 12)) ==
                typeof(InvalidOperationException).GetConstructor(new[] { typeof(string) }), "specific compiled throw mutation");
    }
    static void VerifyConstant(MethodInfo method, bool mutation)
    {
        byte[] bytes = Body(method);
        Require(!method.IsStatic && !method.IsGenericMethod && method.GetParameters().Length == 0 &&
            method.ReturnType == typeof(int) && bytes.Length == 2 && bytes[0] == (mutation ? 0x17 : 0x16) && bytes[1] == 0x2A,
            "constant direct body, no fields, calls or null entry check");
    }
    static Action<object> Direct(ConstructorInfo ctor)
    {
        var bridge = new DynamicMethod("direct_constructor", typeof(void), new[] { typeof(object) }, typeof(FrozenNamingSingletons).Module, true);
        var il = bridge.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Castclass, ctor.DeclaringType);
        il.Emit(OpCodes.Call, ctor);
        il.Emit(OpCodes.Ret);
        return bridge.CreateDelegate<Action<object>>();
    }
    static Func<object, int> Direct(MethodInfo method)
    {
        var bridge = new DynamicMethod("direct_constant", typeof(int), new[] { typeof(object) }, typeof(FrozenNamingSingletons).Module, true);
        var il = bridge.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Castclass, method.DeclaringType);
        il.Emit(OpCodes.Call, method); // Deliberately not callvirt; null this is a CIL direct-call case.
        il.Emit(OpCodes.Ret);
        return bridge.CreateDelegate<Func<object, int>>();
    }
    sealed class StaticSnapshot
    {
        readonly FieldInfo[] fields;
        readonly object[] values;
        public StaticSnapshot(Type type)
        {
            fields = type.GetFields(Members);
            Require(fields.All(f => f.IsStatic && !f.FieldType.IsValueType), "reference-only static fields");
            values = fields.Select(f => f.GetValue(null)).ToArray();
        }
        public void Verify()
        {
            Require(fields.Select((f, i) => ReferenceEquals(f.GetValue(null), values[i])).All(v => v),
                "each side preserves its own static field identities");
        }
    }

    static void Execute(string corpus, string evidencePath, string recoveredPath, bool mutation)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(evidencePath));
        var entry = document.RootElement;
        var catalog = entry.GetProperty("catalog");
        var owner = catalog.GetProperty("owner");
        string token = Text(entry, "token");
        bool constructorJob = token == Text(owner, "constructor");
        string path = Path.Combine(corpus, "assemblies", Text(entry, "assembly"));
        Require(JsonElement.DeepEquals(Inventory(path, Text(owner, "token")), catalog), "repeat independent static safety inventory before load");
        var original = new CorpusContext(corpus).LoadFromAssemblyPath(path);
        Require(original.ManifestModule.ModuleVersionId.ToString() == Text(catalog, "mvid") &&
            original.ManifestModule.GetType("<Module>")?.TypeInitializer == null, "loaded MVID and module initialization");
        var leftType = original.ManifestModule.ResolveType(Id(owner, "token"));
        Require(leftType.FullName.Replace('+', '/') == Text(owner, "name") && leftType.BaseType == typeof(object) &&
            !leftType.ContainsGenericParameters && leftType.GetFields(Members).All(f => f.IsStatic), "loaded owner identity and field-free receiver");
        foreach (var record in owner.GetProperty("methods").EnumerateArray())
        {
            var method = original.ManifestModule.ResolveMethod(Id(record, "token"));
            Require(method.DeclaringType == leftType && method.Name == Text(record, "name") &&
                (int)method.Attributes == record.GetProperty("flags").GetInt32() &&
                (int)method.GetMethodImplementationFlags() == record.GetProperty("implFlags").GetInt32() &&
                Convert.ToHexString(Body(method)) == Text(record, "il"), "PE/reflection scoped method agreement");
        }
        var leftCtor = (ConstructorInfo)original.ManifestModule.ResolveMethod(Id(owner, "constructor"));
        var leftField = original.ManifestModule.ResolveField(Id(owner, "singleton"));
        Require(leftField.DeclaringType == leftType && leftField.FieldType == leftType && leftField.IsStatic && leftField.IsInitOnly &&
            leftType.TypeInitializer.MetadataToken == Id(owner, "initializer"), "loaded initializer and own singleton target");
        VerifyConstructor(leftCtor, false);
        var recovered = new AssemblyLoadContext("recovered", true).LoadFromAssemblyPath(recoveredPath);
        Require(recovered.GetReferencedAssemblies().All(a => a.Name == "System.Runtime" || a.Name == "System.Private.CoreLib"),
            "framework-only recovered references; no original call/proxy");
        var rightType = recovered.GetType(Text(entry, "recoveredType"), true);
        Require(recovered.GetTypes().Select(t => t.FullName).OrderBy(n => n).SequenceEqual(
            new[] { "SingletonShell", rightType.FullName }.OrderBy(n => n)) &&
            recovered.GetTypes().All(t => t.BaseType == typeof(object) && t.TypeInitializer == null && !t.ContainsGenericParameters),
            "only minimal ordinary shell types without initializer");
        var rightCtor = rightType.GetConstructors(Members).Single();
        Require(!rightCtor.IsStatic && rightCtor.IsPublic && rightCtor.GetParameters().Length == 0, "actual constructor ABI");
        var rightField = rightType.GetField(Text(entry, "singletonName"), Members);
        Require(rightType.GetFields(Members).Length == 1 && rightField != null && rightField.IsStatic && !rightField.IsInitOnly &&
            rightField.FieldType == rightType && recovered.GetType("SingletonShell").GetFields(Members).Length == 0,
            "explicit writable test fixture singleton only");
        VerifyConstructor(rightCtor, mutation && constructorJob);
        Require(rightType.GetMethods(Members).Length == (entry.TryGetProperty("constantName", out _) ? 1 : 0), "only selected recovered members");
        var leftCall = Direct(leftCtor);
        var rightCall = Direct(rightCtor);
        MethodInfo leftMethod = null, rightMethod = null;
        Func<object, int> leftConstant = null, rightConstant = null;
        if (!constructorJob)
        {
            leftMethod = (MethodInfo)original.ManifestModule.ResolveMethod(Convert.ToInt32(token[2..], 16));
            rightMethod = rightType.GetMethod(Text(entry, "constantName"), Members);
            Require(leftMethod.DeclaringType == leftType && rightMethod != null && rightMethod.IsAssembly && !rightMethod.IsVirtual,
                "actual internal constant method ABI");
            VerifyConstant(leftMethod, false);
            VerifyConstant(rightMethod, mutation);
            leftConstant = Direct(leftMethod);
            rightConstant = Direct(rightMethod);
        }
        // This is reference setup only. The recovered assembly has no initializer
        // and never imports the original singleton or any original poststate.
        RuntimeHelpers.RunClassConstructor(leftType.TypeHandle);
        object leftSingleton = leftField.GetValue(null);
        Require(leftSingleton != null && leftSingleton.GetType() == leftType, "original oracle setup succeeded");
        var beforeLeft = new StaticSnapshot(leftType);
        void Mismatch(string kind, string phase, int pairs, Exception failure = null)
        {
            beforeLeft.Verify();
            Print(new { status = "behavior-mismatch", token, side = "recovered", kind, phase, completedPairs = pairs,
                exception = failure?.GetType().FullName, message = failure?.Message,
                selectedConstructor = Text(owner, "constructor"), originalOracleSetupSucceeded = true,
                recoveredConstructorIl = Convert.ToHexString(Body(rightCtor)),
                recoveredMethodIl = rightMethod == null ? null : Convert.ToHexString(Body(rightMethod)),
                originalInitializerIl = Convert.ToHexString(Body(leftType.TypeInitializer)) });
            Environment.ExitCode = 42;
        }
        object rightSingleton = RuntimeHelpers.GetUninitializedObject(rightType);
        try { rightCall(rightSingleton); }
        catch (InvalidOperationException failure) when (mutation && constructorJob && failure.Message == MutationMessage)
        {
            Mismatch("recovered-constructor-threw", "recovered-preinitialization", 0, failure);
            return;
        }
        rightField.SetValue(null, rightSingleton);
        Require(!ReferenceEquals(leftSingleton, rightSingleton) && ReferenceEquals(rightField.GetValue(null), rightSingleton),
            "independent own singleton prepared with actual recovered constructor");
        var beforeRight = new StaticSnapshot(rightType);
        void Preserved()
        {
            beforeLeft.Verify();
            beforeRight.Verify();
            Require(ReferenceEquals(leftField.GetValue(null), leftSingleton) && ReferenceEquals(rightField.GetValue(null), rightSingleton),
                "each side preserves its own preinitialized singleton identity");
        }
        int pairs = 0;
        bool Pair(object left, object right, string phase)
        {
            if (constructorJob) { leftCall(left); rightCall(right); }
            else
            {
                int expected = leftConstant(left), actual = rightConstant(right);
                Require(expected == 0, "original constant returns zero including null this");
                if (actual != expected)
                {
                    Preserved();
                    Mismatch("return-value", phase, pairs);
                    return false;
                }
            }
            Preserved();
            pairs++;
            return true;
        }
        foreach (bool warmed in new[] { false, true })
        {
            for (int sample = 0; sample < 2; sample++)
            {
                object left = RuntimeHelpers.GetUninitializedObject(leftType), right = RuntimeHelpers.GetUninitializedObject(rightType);
                Require(!ReferenceEquals(left, right) && !ReferenceEquals(left, leftSingleton) && !ReferenceEquals(right, rightSingleton),
                    "fresh independent receiver pair, no oracle state copying");
                if (warmed) { leftCall(left); rightCall(right); }
                for (int repeat = 0; repeat < 3; repeat++)
                    if (!Pair(left, right, warmed ? "constructor-warmed-receiver" : "fresh-receiver")) return;
            }
        }
        for (int repeat = 0; repeat < 3; repeat++) if (!Pair(leftSingleton, rightSingleton, "own-singleton")) return;
        if (!constructorJob)
            for (int repeat = 0; repeat < 3; repeat++) if (!Pair(null, null, "null-this-direct-call")) return;
        Require(!mutation, "compiled mutation must fail behaviorally");
        Print(new { status = "passed", token, pairs, freshPairs = 6, constructorWarmedPairs = 6, singletonPairs = 3,
            nullThisPairs = constructorJob ? 0 : 3, repeats = 3, originalOracleSetupSucceeded = true,
            recoveredIndependentSetupSucceeded = true, singletonIdentityPreserved = true, originalStateCopied = false,
            originalStaticFields = leftType.GetFields(Members).Select(f => f.Name).ToArray(),
            recoveredConstructorIl = Convert.ToHexString(Body(rightCtor)),
            recoveredMethodIl = rightMethod == null ? null : Convert.ToHexString(Body(rightMethod)),
            originalInitializerIl = Convert.ToHexString(Body(leftType.TypeInitializer)),
            limits = "Post-preinitialization field-free fragments only; warmed receivers have no instance fields. No full startup or type-initializer equivalence; no constructor null-this claim." });
    }
    static void Main(string[] args)
    {
        Require(args.Length == 3 && args[0] == "catalog" || (args.Length == 4 || args.Length == 5 && args[4] == "mutation") &&
            args[0] == "execute", "command arguments");
        if (args[0] == "catalog") Print(Inventory(args[1], args[2]));
        else Execute(args[1], args[2], args[3], args.Length == 5);
    }
}
