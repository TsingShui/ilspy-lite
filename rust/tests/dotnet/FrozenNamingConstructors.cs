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

// Static discovery never loads the corpus. Execution permits only independently
// verified field-free constructors, with the original confined to the oracle side.
internal static class FrozenNamingConstructors
{
    const BindingFlags Members = BindingFlags.Public | BindingFlags.NonPublic |
        BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;

    static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException("Invalid gate input: " + message);
    }

    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";
    static void Print(object value) => Console.WriteLine(JsonSerializer.Serialize(value));

    // Nops are permitted only at instruction boundaries, never inside operands.
    static int? ObjectCall(byte[] bytes)
    {
        int offset = 0;
        bool Take(byte code)
        {
            while (offset < bytes.Length && bytes[offset] == 0) offset++;
            return offset < bytes.Length && bytes[offset++] == code;
        }
        if (!Take(0x02) || !Take(0x28) || offset + 4 > bytes.Length) return null;
        int target = BitConverter.ToInt32(bytes, offset);
        offset += 4;
        if (!Take(0x2A)) return null;
        while (offset < bytes.Length && bytes[offset] == 0) offset++;
        return offset == bytes.Length ? target : null;
    }

    static void Catalog(string path, string rootToken)
    {
        using var pe = new PEReader(File.OpenRead(path));
        MetadataReader md = pe.GetMetadataReader();
        string[] TypePath(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parent = type.GetDeclaringType();
            return (parent.IsNil ? Array.Empty<string>() : TypePath(parent))
                .Append(md.GetString(type.Name)).ToArray();
        }
        string Namespace(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parent = type.GetDeclaringType();
            return parent.IsNil ? md.GetString(type.Namespace) : Namespace(parent);
        }
        string Name(TypeDefinitionHandle handle)
        {
            string ns = Namespace(handle);
            return (ns.Length == 0 ? "" : ns + ".") + string.Join("/", TypePath(handle));
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
        bool CoreConstructor(int token)
        {
            var handle = MetadataTokens.EntityHandle(token);
            if (handle.Kind != HandleKind.MemberReference) return false;
            var member = md.GetMemberReference((MemberReferenceHandle)handle);
            return md.GetString(member.Name) == ".ctor" &&
                Convert.ToHexString(md.GetBlobBytes(member.Signature)) == "200001" && CoreObject(member.Parent);
        }
        object Method(MethodDefinitionHandle handle)
        {
            var method = md.GetMethodDefinition(handle);
            var body = method.RelativeVirtualAddress == 0 ? null : pe.GetMethodBody(method.RelativeVirtualAddress);
            byte[] bytes = body?.GetILBytes() ?? Array.Empty<byte>();
            int? call = ObjectCall(bytes);
            return new { token = Token(handle), name = md.GetString(method.Name),
                signature = Convert.ToHexString(md.GetBlobBytes(method.Signature)), flags = (int)method.Attributes,
                il = Convert.ToHexString(bytes), eh = body?.ExceptionRegions.Length ?? 0,
                objectOnly = body != null && body.ExceptionRegions.Length == 0 && call.HasValue && CoreConstructor(call.Value),
                coreCallToken = call.HasValue ? $"0x{call.Value:X8}" : null };
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
            var parent = type.GetDeclaringType();
            return type.GetGenericParameters().Count != 0 || !parent.IsNil && Generic(parent);
        }
        var types = md.TypeDefinitions.Where(Owned).Select(handle => {
            var type = md.GetTypeDefinition(handle);
            var methods = type.GetMethods().ToArray();
            return new { token = Token(handle), name = Name(handle), path = TypePath(handle),
                parent = Token(type.GetDeclaringType()), generic = Generic(handle), objectBase = CoreObject(type.BaseType),
                finalizer = methods.Any(m => md.GetString(md.GetMethodDefinition(m).Name) == "Finalize"),
                initializers = methods.Where(m => md.GetString(md.GetMethodDefinition(m).Name) == ".cctor").Select(Method).ToArray(),
                fields = type.GetFields().Select(h => {
                    var field = md.GetFieldDefinition(h);
                    return new { token = Token(h), name = md.GetString(field.Name), flags = (int)field.Attributes,
                        signature = Convert.ToHexString(md.GetBlobBytes(field.Signature)) };
                }).ToArray(), methods = methods.Select(Method).ToArray() };
        }).ToArray();
        Print(new { mvid = md.GetGuid(md.GetModuleDefinition().Mvid).ToString(), types,
            moduleInitializers = md.TypeDefinitions.Where(h => md.GetString(md.GetTypeDefinition(h).Name) == "<Module>")
                .SelectMany(h => md.GetTypeDefinition(h).GetMethods())
                .Where(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor").Select(Method).ToArray() });
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

    static byte[] Bytes(ConstructorInfo constructor)
    {
        var body = constructor.GetMethodBody();
        Require(body != null && body.ExceptionHandlingClauses.Count == 0, "ordinary constructor body without EH");
        return body.GetILAsByteArray();
    }

    static bool CoreTarget(Module module, int token) => module.ResolveMethod(token) is ConstructorInfo target &&
        target.DeclaringType == typeof(object) && !target.IsStatic && target.GetParameters().Length == 0;

    static void Verify(ConstructorInfo constructor)
    {
        int? call = ObjectCall(Bytes(constructor));
        Require(call.HasValue && CoreTarget(constructor.Module, call.Value), "exact runtime Object direct-call protocol");
    }

    static void VerifyMutation(ConstructorInfo constructor)
    {
        // Release csc emits exactly Object::.ctor, ldstr, exception::.ctor, throw.
        // This is validated, then executed; protocol rejection is not a mutation win.
        byte[] bytes = Bytes(constructor);
        Require(bytes.Length == 17 && bytes[0] == 0x02 && bytes[1] == 0x28 && bytes[6] == 0x72 &&
            bytes[11] == 0x73 && bytes[16] == 0x7A, "specific compiled throw mutation shape");
        Require(CoreTarget(constructor.Module, BitConverter.ToInt32(bytes, 2)), "mutation core Object target");
        Require(constructor.Module.ResolveString(BitConverter.ToInt32(bytes, 7)) == "frozen-ctor-mutation", "mutation literal");
        Require(constructor.Module.ResolveMethod(BitConverter.ToInt32(bytes, 12)) ==
            typeof(InvalidOperationException).GetConstructor(new[] { typeof(string) }), "exact core exception target");
    }

    static Action<object> Direct(ConstructorInfo constructor)
    {
        var method = new DynamicMethod("direct_constructor", typeof(void), new[] { typeof(object) },
            typeof(FrozenNamingConstructors).Module, true);
        var il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Castclass, constructor.DeclaringType);
        il.Emit(OpCodes.Call, constructor);
        il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<object>>();
    }

    static void Execute(string corpus, string evidencePath, string recoveredPath, bool mutation)
    {
        using var evidence = JsonDocument.Parse(File.ReadAllText(evidencePath));
        var entry = evidence.RootElement;
        string Text(string key) => entry.GetProperty(key).GetString();
        int Id(string key) => Convert.ToInt32(Text(key)[2..], 16);
        var context = new CorpusContext(corpus);
        var original = context.LoadFromAssemblyPath(Path.Combine(corpus, "assemblies", Text("assembly")));
        Require(original.ManifestModule.ModuleVersionId.ToString() == Text("mvid"), "original MVID");
        var leftCtor = original.ManifestModule.ResolveMethod(Id("token")) as ConstructorInfo;
        Require(leftCtor != null && !leftCtor.IsStatic && leftCtor.GetParameters().Length == 0 &&
            leftCtor.DeclaringType.MetadataToken == Id("owner") &&
            leftCtor.DeclaringType.FullName.Replace('+', '/') == Text("ownerName"), "original scoped method identity");
        var leftType = leftCtor.DeclaringType;
        Require(leftType.BaseType == typeof(object) && !leftType.ContainsGenericParameters && leftType.TypeInitializer == null &&
            !leftType.GetMethods(Members).Any(m => m.Name == "Finalize"), "safe original initialization/ancestry");
        Require(original.ManifestModule.GetType("<Module>")?.TypeInitializer == null, "no module initializer");
        Require(Convert.ToHexString(Bytes(leftCtor)) == Text("il"), "independent PE/reflection IL agreement");
        Verify(leftCtor);
        var recovered = new AssemblyLoadContext("recovered", true).LoadFromAssemblyPath(recoveredPath);
        Require(recovered.GetReferencedAssemblies().All(a => a.Name == "System.Runtime" || a.Name == "System.Private.CoreLib"),
            "recovered has only framework references, no selected-original proxy");
        Require(recovered.GetTypes().Length == 2, "only selected instance shell and ordinary outer context");
        var rightType = recovered.GetType(Text("recoveredType"), throwOnError: true);
        Require(rightType.BaseType == typeof(object) && rightType.TypeInitializer == null && rightType.GetFields(Members).Length == 0,
            "field-free instance shell");
        var rightCtor = rightType.GetConstructor(Members, null, Type.EmptyTypes, null);
        Require(rightCtor != null && rightType.GetConstructors(Members).Length == 1, "unique actual parameterless constructor");
        if (mutation) VerifyMutation(rightCtor); else Verify(rightCtor);
        Action<object> left = Direct(leftCtor), right = Direct(rightCtor);
        var fields = leftType.GetFields(Members).Where(f => !f.IsStatic).ToArray();
        // Inspect primitive values and reference identity only. Unknown struct
        // values never invoke user Equals, constructors, or gameplay callbacks.
        var observed = fields.Where(f => !f.FieldType.IsValueType || f.FieldType.IsPrimitive).ToArray();
        int pairs = 0;
        foreach (int seed in new[] { -1, 0, 1, int.MaxValue })
        {
            object originalReceiver = RuntimeHelpers.GetUninitializedObject(leftType);
            object recoveredReceiver = RuntimeHelpers.GetUninitializedObject(rightType);
            Require(!ReferenceEquals(originalReceiver, recoveredReceiver), "independent receivers");
            foreach (var field in observed.Where(f => f.FieldType == typeof(int) && !f.IsInitOnly))
                field.SetValue(originalReceiver, seed);
            object[] before = observed.Select(f => f.GetValue(originalReceiver)).ToArray();
            for (int repeat = 0; repeat < 2; repeat++)
            {
                left(originalReceiver);
                // The original is a reference only. Its poststate is never copied
                // to a recovered receiver, field, initializer, or implementation.
                for (int i = 0; i < observed.Length; i++)
                {
                    object after = observed[i].GetValue(originalReceiver);
                    Require(observed[i].FieldType.IsPrimitive ? Equals(before[i], after) : ReferenceEquals(before[i], after),
                        "original constructor preserves observed state");
                }
                try { right(recoveredReceiver); }
                catch (InvalidOperationException failure) when (mutation && failure.Message == "frozen-ctor-mutation")
                {
                    Print(new { status = "behavior-mismatch", kind = "recovered-constructor-threw",
                        exception = failure.GetType().FullName, message = failure.Message, token = Text("token"),
                        originalReturned = true, independentReceivers = true });
                    Environment.ExitCode = 42;
                    return;
                }
                pairs++;
            }
        }
        Require(!mutation, "compiled mutation must throw at runtime");
        Print(new { status = "passed", pairs, token = Text("token"), observedOriginalFields = observed.Select(f => f.Name).ToArray(),
            unobservedOriginalFields = fields.Except(observed).Select(f => f.Name).ToArray(),
            originalStatePreserved = true, recoveredFieldsOmitted = true, noOriginalProxy = true });
    }

    static void Main(string[] args)
    {
        Require(args.Length == 3 && args[0] == "catalog" ||
            (args.Length == 4 || args.Length == 5 && args[4] == "mutation") && args[0] == "execute", "command arguments");
        if (args[0] == "catalog") Catalog(args[1], args[2]);
        else Execute(args[1], args[2], args[3], args.Length == 5);
    }
}
