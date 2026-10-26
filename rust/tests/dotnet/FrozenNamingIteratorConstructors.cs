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
using System.Threading;

internal static class FrozenNamingIteratorConstructors
{
    const BindingFlags Members = BindingFlags.Public | BindingFlags.NonPublic |
        BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly;
    static void Require(bool value, string reason)
    {
        if (!value) throw new InvalidOperationException("Invalid gate input: " + reason);
    }
    static void Print(object value) => Console.WriteLine(JsonSerializer.Serialize(value));
    static string Text(JsonElement value, string name) => value.GetProperty(name).GetString();
    static int Id(JsonElement value, string name) => Convert.ToInt32(Text(value, name)[2..], 16);
    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";

    static void Catalog(string path, string token)
    {
        using var pe = new PEReader(File.OpenRead(path));
        var md = pe.GetMetadataReader();
        var handle = (MethodDefinitionHandle)MetadataTokens.EntityHandle(Convert.ToInt32(token[2..], 16));
        var method = md.GetMethodDefinition(handle);
        var owner = method.GetDeclaringType();
        var type = md.GetTypeDefinition(owner);
        var body = pe.GetMethodBody(method.RelativeVirtualAddress);
        byte[] il = body.GetILBytes();
        Require(Convert.ToHexString(md.GetBlobBytes(method.Signature)) == "20010108" && body.ExceptionRegions.Length == 0,
            "ordinary instance void constructor(I4)");
        Require(il.Length == 25 && il[0] == 2 && il[1] == 0x28 && il[6] == 2 && il[7] == 3 &&
            il[8] == 0x7D && il[13] == 2 && il[14] == 0x28 && il[19] == 0x7D && il[24] == 0x2A,
            "exact Object/state/thread constructor protocol");
        bool CoreType(EntityHandle h, string expected)
        {
            if (h.Kind != HandleKind.TypeReference) return false;
            var t = md.GetTypeReference((TypeReferenceHandle)h);
            if (md.GetString(t.Name) != expected || md.GetString(t.Namespace) != "System" ||
                t.ResolutionScope.Kind != HandleKind.AssemblyReference) return false;
            var a = md.GetAssemblyReference((AssemblyReferenceHandle)t.ResolutionScope);
            return md.GetString(a.Name) == "System.Runtime" && a.Version == new Version(6, 0, 0, 0) && a.Flags == 0 &&
                md.GetString(a.Culture) == "" && Convert.ToHexString(md.GetBlobBytes(a.PublicKeyOrToken)) == "B03F5F7F11D50A3A";
        }
        var baseCall = md.GetMemberReference((MemberReferenceHandle)MetadataTokens.EntityHandle(BitConverter.ToInt32(il, 2)));
        var getter = md.GetMemberReference((MemberReferenceHandle)MetadataTokens.EntityHandle(BitConverter.ToInt32(il, 15)));
        Require(md.GetString(baseCall.Name) == ".ctor" && CoreType(baseCall.Parent, "Object") &&
            Convert.ToHexString(md.GetBlobBytes(baseCall.Signature)) == "200001", "exact pinned core Object call");
        Require(md.GetString(getter.Name) == "get_CurrentManagedThreadId" && CoreType(getter.Parent, "Environment") &&
            Convert.ToHexString(md.GetBlobBytes(getter.Signature)) == "000008", "exact pinned core thread getter");
        string[] TypePath(TypeDefinitionHandle h)
        {
            var t = md.GetTypeDefinition(h);
            return (t.GetDeclaringType().IsNil ? Array.Empty<string>() : TypePath(t.GetDeclaringType()))
                .Append(md.GetString(t.Name)).ToArray();
        }
        string Namespace(TypeDefinitionHandle h)
        {
            var t = md.GetTypeDefinition(h);
            return t.GetDeclaringType().IsNil ? md.GetString(t.Namespace) : Namespace(t.GetDeclaringType());
        }
        Require(CoreType(type.BaseType, "Object") && type.GetGenericParameters().Count == 0 &&
            !type.GetMethods().Any(m => md.GetString(md.GetMethodDefinition(m).Name) is ".cctor" or "Finalize"), "safe owner context");
        Require(!md.TypeDefinitions.Where(h => md.GetString(md.GetTypeDefinition(h).Name) == "<Module>")
            .SelectMany(h => md.GetTypeDefinition(h).GetMethods()).Any(h => md.GetString(md.GetMethodDefinition(h).Name) == ".cctor"),
            "no module initializer");
        object Field(int at)
        {
            var h = (FieldDefinitionHandle)MetadataTokens.EntityHandle(BitConverter.ToInt32(il, at));
            var f = md.GetFieldDefinition(h);
            Require(f.GetDeclaringType() == owner && Convert.ToHexString(md.GetBlobBytes(f.Signature)) == "0608" &&
                (f.Attributes & FieldAttributes.Static) == 0, "own I4 field");
            return new { token = Token(h), name = md.GetString(f.Name), flags = (int)f.Attributes };
        }
        Print(new { mvid = md.GetGuid(md.GetModuleDefinition().Mvid).ToString(), owner = Token(owner),
            ownerName = Namespace(owner) + "." + string.Join("/", TypePath(owner)), path = TypePath(owner),
            token, signature = Convert.ToHexString(md.GetBlobBytes(method.Signature)), flags = (int)method.Attributes,
            il = Convert.ToHexString(il), state = Field(9), thread = Field(20),
            getterToken = $"0x{BitConverter.ToInt32(il, 15):X8}" });
    }

    sealed class CorpusContext : AssemblyLoadContext
    {
        readonly string root;
        public CorpusContext(string root) : base(isCollectible: true) { this.root = root; }
        protected override Assembly Load(AssemblyName name)
        {
            foreach (string directory in new[] { "assemblies", "runtime-deps" })
            {
                string path = Path.Combine(root, directory, name.Name + ".dll");
                if (File.Exists(path)) return LoadFromAssemblyPath(path);
            }
            return null;
        }
    }
    static readonly MethodInfo ThreadGetter = typeof(Environment).GetProperty(nameof(Environment.CurrentManagedThreadId)).GetMethod;
    static readonly Dictionary<short, OpCode> Opcodes = typeof(OpCodes).GetFields().Where(f => f.FieldType == typeof(OpCode))
        .Select(f => (OpCode)f.GetValue(null)).ToDictionary(o => o.Value);
    static void Verify(ConstructorInfo ctor, FieldInfo state, FieldInfo thread)
    {
        var body = ctor.GetMethodBody();
        Require(body != null && body.ExceptionHandlingClauses.Count == 0, "ordinary loaded body");
        byte[] bytes = body.GetILAsByteArray();
        var effects = new List<string>();
        for (int at = 0; at < bytes.Length;)
        {
            short value = bytes[at++];
            if (value == 0xFE) value = (short)(0xFE00 | bytes[at++]);
            Require(Opcodes.TryGetValue(value, out var op), "known compiler opcode");
            int size = op.OperandType switch { OperandType.InlineNone => 0,
                OperandType.ShortInlineI or OperandType.ShortInlineVar or OperandType.ShortInlineBrTarget => 1,
                OperandType.InlineVar => 2, OperandType.InlineMethod or OperandType.InlineField or OperandType.InlineI => 4,
                _ => throw new InvalidOperationException("Unexpected constructor operand") };
            Require(at + size <= bytes.Length, "complete compiler operand");
            if (op == OpCodes.Call)
            {
                var target = ctor.Module.ResolveMethod(BitConverter.ToInt32(bytes, at));
                if (target is ConstructorInfo c && c.DeclaringType == typeof(object) && c.GetParameters().Length == 0) effects.Add("Object");
                else { Require(target == ThreadGetter, "exact runtime core thread getter, no proxy"); effects.Add("Thread"); }
            }
            else if (op == OpCodes.Stfld)
            {
                var field = ctor.Module.ResolveField(BitConverter.ToInt32(bytes, at));
                Require(field == state || field == thread, "only observed field stores");
                effects.Add(field == state ? "State" : "ThreadStore");
            }
            else Require(new[] { "nop", "ret", "ldarg.0", "ldarg.1", "ldloc.0", "stloc.0", "ldc.i4.0", "sub", "pop" }
                .Contains(op.Name), "bounded constructor IL without extra calls/fields/control flow");
            at += size;
        }
        Require(effects.SequenceEqual(new[] { "Object", "State", "Thread", "ThreadStore" }), "exact call/store effect order and counts");
    }
    static Action<object, int> Direct(ConstructorInfo constructor)
    {
        var method = new DynamicMethod("direct_ctor", typeof(void), new[] { typeof(object), typeof(int) },
            typeof(FrozenNamingIteratorConstructors).Module, true);
        var il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Castclass, constructor.DeclaringType); il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Call, constructor); il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<object, int>>();
    }
    sealed class Mismatch : Exception
    {
        public readonly string Kind;
        public Mismatch(string kind, string context) : base(context) { Kind = kind; }
    }
    static void Check(bool value, string kind, string context) { if (!value) throw new Mismatch(kind, context); }
    static void Execute(string corpus, string evidencePath, string recoveredPath)
    {
        using var document = JsonDocument.Parse(File.ReadAllText(evidencePath));
        var entry = document.RootElement;
        var context = new CorpusContext(corpus);
        var original = context.LoadFromAssemblyPath(Path.Combine(corpus, "assemblies", Text(entry, "assembly")));
        Require(original.ManifestModule.ModuleVersionId.ToString() == Text(entry, "mvid"), "original MVID");
        var left = (ConstructorInfo)original.ManifestModule.ResolveMethod(Id(entry, "token"));
        Require(left.DeclaringType.MetadataToken == Id(entry, "owner") &&
            left.DeclaringType.FullName.Replace('+', '/') == Text(entry, "ownerName") &&
            left.GetParameters().Select(p => p.ParameterType).SequenceEqual(new[] { typeof(int) }) &&
            (int)left.Attributes == entry.GetProperty("flags").GetInt32() &&
            Convert.ToHexString(left.GetMethodBody().GetILAsByteArray()) == Text(entry, "il"), "loaded identity/signature/IL");
        Require(left.DeclaringType.TypeInitializer == null && left.DeclaringType.BaseType == typeof(object), "no original owner initialization");
        var assembly = new AssemblyLoadContext("recovered", true).LoadFromAssemblyPath(recoveredPath);
        Require(assembly.GetReferencedAssemblies().All(a => a.Name == "System.Runtime" || a.Name == "System.Private.CoreLib"), "framework-only recovered implementation");
        var type = assembly.GetType(Text(entry, "recoveredType"), true);
        Require(type.TypeInitializer == null && type.BaseType == typeof(object) && type.GetFields(Members).Length == 2, "bounded two-I4 shadow");
        var right = type.GetConstructor(Members, null, new[] { typeof(int) }, null);
        var stateRecord = entry.GetProperty("state"); var threadRecord = entry.GetProperty("thread");
        var leftState = original.ManifestModule.ResolveField(Id(stateRecord, "token"));
        var leftThread = original.ManifestModule.ResolveField(Id(threadRecord, "token"));
        var rightState = type.GetField(Text(stateRecord, "alias"), Members); var rightThread = type.GetField(Text(threadRecord, "alias"), Members);
        Require(leftState.FieldType == typeof(int) && leftThread.FieldType == typeof(int) && rightState.FieldType == typeof(int) &&
            rightThread.FieldType == typeof(int) && !leftState.IsStatic && !leftThread.IsStatic, "I4 field correspondence");
        Verify(left, leftState, leftThread); Verify(right, rightState, rightThread);
        var invokeLeft = Direct(left); var invokeRight = Direct(right);
        int pairs = 0, nulls = 0; var threads = new List<int>();
        Exception failure = null;
        void Cases()
        {
            try
            {
                int id = Environment.CurrentManagedThreadId; threads.Add(id);
                foreach (int dirty in new[] { -1, 0, int.MaxValue })
                    foreach (int argument in new[] { int.MinValue, int.MinValue + 1, -1, 0, 1, int.MaxValue - 1, int.MaxValue })
                    {
                        object a = RuntimeHelpers.GetUninitializedObject(left.DeclaringType), b = RuntimeHelpers.GetUninitializedObject(type);
                        leftState.SetValue(a, dirty); leftThread.SetValue(a, dirty);
                        rightState.SetValue(b, dirty); rightThread.SetValue(b, dirty);
                        var untouched = left.DeclaringType.GetFields(Members).Where(f => !f.IsStatic && f != leftState && f != leftThread &&
                            (!f.FieldType.IsValueType || f.FieldType.IsPrimitive)).ToArray();
                        var before = untouched.Select(f => f.GetValue(a)).ToArray();
                        for (int repeat = 0; repeat < 3; repeat++)
                        {
                            int value = repeat == 1 ? ~argument : argument;
                            invokeLeft(a, value); invokeRight(b, value);
                            Check((int)leftState.GetValue(a) == value, "original-state", "independent state expectation");
                            Check((int)rightState.GetValue(b) == value, "state-write", "actual recovered state expectation");
                            Check((int)leftThread.GetValue(a) == id, "original-thread", "current execution thread");
                            Check((int)rightThread.GetValue(b) == id, "thread-write", "current execution thread");
                            for (int n = 0; n < untouched.Length; n++)
                            {
                                object after = untouched[n].GetValue(a);
                                Check(untouched[n].FieldType.IsPrimitive ? Equals(before[n], after) : ReferenceEquals(before[n], after),
                                    "original-preserved", untouched[n].Name);
                            }
                            pairs++;
                        }
                    }
                foreach (int argument in new[] { int.MinValue, int.MinValue + 1, -1, 0, 1, int.MaxValue - 1, int.MaxValue })
                {
                    foreach (var invoke in new[] { invokeLeft, invokeRight })
                    {
                        bool threw = false;
                        try { invoke(null, argument); } catch (NullReferenceException) { threw = true; }
                        Check(threw, "null-exception", "null receiver must fault before thread getter");
                    }
                    pairs++; nulls++;
                }
            }
            catch (Exception exception) { failure = exception; }
        }
        Cases();
        if (failure == null) { var worker = new Thread(Cases); worker.Start(); worker.Join(); }
        if (failure is Mismatch mismatch)
        {
            Print(new { status = "behavior-mismatch", kind = mismatch.Kind, detail = mismatch.Message, token = Text(entry, "token") });
            Environment.ExitCode = 42; return;
        }
        if (failure != null) throw failure;
        Require(threads.Count == 2 && threads[0] != threads[1], "two distinct runtime thread contexts");
        Print(new { status = "passed", pairs, nullPairs = nulls, threads, token = Text(entry, "token"), noOriginalProxy = true,
            limits = "Two I4 fields and observed original state only; no full iterator/class ABI or cross-runtime claim" });
    }
    static void Main(string[] args)
    {
        Require(args.Length == 3 && args[0] == "catalog" || args.Length == 4 && args[0] == "execute", "command arguments");
        if (args[0] == "catalog") Catalog(args[1], args[2]); else Execute(args[1], args[2], args[3]);
    }
}
