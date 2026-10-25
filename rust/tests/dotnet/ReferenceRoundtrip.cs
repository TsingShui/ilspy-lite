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
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

// Test-only: original methods are invoked with an exact IL call. Recovered bodies
// execute on separately generated state shells, never delegated to the original.
internal static class ReferenceRoundtrip
{
    record Entry(int Token, string Source);
    record Job(string Kind, string Assembly, string Dependencies, Entry[] Methods);
    static readonly Dictionary<int, Func<object, object[], object>> OriginalCalls = new();
    static int checks;
    static string TypeName(Type type) => type == typeof(void) ? "void" : type == typeof(int) ? "int" :
        type == typeof(bool) ? "bool" : type == typeof(long) ? "long" : type == typeof(uint) ? "uint" :
        type == typeof(ulong) ? "ulong" : type == typeof(nint) ? "nint" : type == typeof(nuint) ? "nuint" :
        type.IsSZArray ? TypeName(type.GetElementType()!) + "[]" : "global::" + type.FullName;
    static string Alias(FieldInfo field) => Regex.IsMatch(field.Name, @"^[A-Za-z_][A-Za-z0-9_]*$") && !field.Name.StartsWith("__il_field_", StringComparison.Ordinal)
        ? "@" + field.Name : $"__il_field_{field.MetadataToken:X8}";
    static FieldInfo Field(Type type, string name) => type.GetField(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static)
        ?? throw new Exception($"Missing field {type}.{name}");

    static void Main(string[] args)
    {
        Job job = JsonSerializer.Deserialize<Job>(File.ReadAllText(args[1]), new JsonSerializerOptions { PropertyNameCaseInsensitive = true })!;
        var originalContext = new AssemblyLoadContext("original", isCollectible: true);
        originalContext.Resolving += (context, name) => {
            string path = Path.Combine(job.Dependencies, name.Name + ".dll");
            return File.Exists(path) ? context.LoadFromAssemblyPath(path) : null;
        };
        Assembly original = originalContext.LoadFromAssemblyPath(job.Assembly);
        var methods = job.Methods.ToDictionary(e => e.Token, e => (MethodInfo)original.ManifestModule.ResolveMethod(e.Token)!);
        if (args[0] == "prepare")
        {
            Prepare(job, original, methods, args[2]);
            Console.WriteLine($"Prepared {methods.Count} recovered bodies; no original method executed.");
            return;
        }
        if (args[0] != "run") throw new Exception("Unknown mode");
        Assembly recovered = new AssemblyLoadContext("recovered", isCollectible: true).LoadFromAssemblyPath(args[2]);
        if (job.Kind == "openra") OpenRa(original, recovered, methods);
        else if (job.Kind == "arrays") Arrays(recovered, methods);
        else if (job.Kind == "native") Native(recovered, methods);
        else if (job.Kind == "conversions") Conversions(recovered, methods);
        else if (job.Kind == "addresses") Addresses(recovered, methods);
        else if (job.Kind == "references") References(original, recovered, methods);
        else throw new Exception("Unknown corpus kind");
        Console.WriteLine($"{job.Kind}: {checks} return/exception/state comparisons passed");
    }

    static void Prepare(Job job, Assembly original, Dictionary<int, MethodInfo> methods, string output)
    {
        var fields = new Dictionary<int, FieldInfo>();
        var types = new HashSet<Type>();
        var code = new List<string>();
        foreach (Entry entry in job.Methods)
        {
            MethodInfo method = methods[entry.Token];
            types.Add(method.DeclaringType!);
            string source = File.ReadAllText(entry.Source);
            foreach (Match match in Regex.Matches(source, @"// field token 0x([0-9A-F]{8})"))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                FieldInfo field = original.ManifestModule.ResolveField(token)!;
                fields[token] = field;
                types.Add(field.DeclaringType!);
                if (field.FieldType.Assembly == original) types.Add(field.FieldType);
            }
            foreach (Match match in Regex.Matches(source, @"__cil_direct_([0-9A-F]{8})\("))
                if (!methods.ContainsKey(Convert.ToInt32(match.Groups[1].Value, 16)))
                    throw new Exception("A direct-call dependency was not recovered");
            string declaration = "@" + method.Name + "(";
            if (source.Split(declaration).Length != 2) throw new Exception("Ambiguous method declaration");
            code.Add(source.Replace(declaration, $"M_{entry.Token:X8}("));
        }
        var shells = new List<string>();
        foreach (Type type in types)
        {
            if (type.IsGenericType || type.IsNested || type.IsValueType) throw new Exception("Unsupported test shell shape");
            string parent = type.BaseType != null && types.Contains(type.BaseType) ? " : " + TypeName(type.BaseType) : "";
            string members = string.Join("\n", fields.Values.Where(f => f.DeclaringType == type).Select(f =>
                $"public {(f.IsStatic ? "static " : "")}{TypeName(f.FieldType)} {Alias(f)};"));
            // Static call adapters forward only to another recovered body.
            foreach (MethodInfo method in methods.Values.Where(m => m.IsStatic && m.DeclaringType == type))
            {
                string parameters = string.Join(", ", method.GetParameters().Select((p, i) => TypeName(p.ParameterType) + " a" + i));
                string arguments = string.Join(", ", method.GetParameters().Select((p, i) => "a" + i));
                members += $"\npublic static {TypeName(method.ReturnType)} @{method.Name}({parameters}) => global::Recovered.M_{method.MetadataToken:X8}({arguments});";
            }
            shells.Add($"namespace {type.Namespace} {{ public class {type.Name}{parent} {{ {members} }} }}");
        }
        foreach (var pair in methods)
        {
            MethodInfo method = pair.Value;
            if (method.IsStatic) continue;
            var parameters = new[] { TypeName(method.DeclaringType!) + " self" }
                .Concat(method.GetParameters().Select((p, i) => TypeName(p.ParameterType) + " a" + i));
            string arguments = string.Join(", ", new[] { "self" }.Concat(method.GetParameters().Select((p, i) => "a" + i)));
            code.Add($"public static {TypeName(method.ReturnType)} __cil_direct_{pair.Key:X8}({string.Join(", ", parameters)}) => M_{pair.Key:X8}({arguments});");
        }
        File.WriteAllText(output, string.Join("\n", shells) + "\npublic static class Recovered {\n" + string.Join("\n", code) + "\n}\n");
    }

    static object Original(MethodInfo method, object receiver, object[] args)
    {
        if (!OriginalCalls.TryGetValue(method.MetadataToken, out var invoke))
        {
            var dm = new DynamicMethod("ExactCall", typeof(object), new[] { typeof(object), typeof(object[]) }, method.Module, true);
            ILGenerator il = dm.GetILGenerator();
            if (!method.IsStatic) { il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Castclass, method.DeclaringType!); }
            foreach (var p in method.GetParameters())
            {
                il.Emit(OpCodes.Ldarg_1); il.Emit(OpCodes.Ldc_I4, p.Position); il.Emit(OpCodes.Ldelem_Ref);
                il.Emit(p.ParameterType.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, p.ParameterType);
            }
            il.Emit(OpCodes.Call, method);
            if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
            else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
            il.Emit(OpCodes.Ret);
            invoke = (Func<object, object[], object>)dm.CreateDelegate(typeof(Func<object, object[], object>));
            OriginalCalls.Add(method.MetadataToken, invoke);
        }
        return invoke(receiver, args);
    }

    static object Outcome(Func<object> action)
    {
        try { return action(); }
        catch (Exception e) { return (e is TargetInvocationException t ? t.InnerException! : e).GetType().FullName; }
    }
    static void Check(Assembly recovered, MethodInfo method, object left, object right, object[] leftArgs, object[] rightArgs,
        Action stateCheck = null, Func<object, object, bool> equal = null)
    {
        object expected = Outcome(() => Original(method, left, leftArgs));
        object actual = Outcome(() => recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!
            .Invoke(null, method.IsStatic ? rightArgs : new[] { right }.Concat(rightArgs).ToArray()));
        if (!(equal?.Invoke(expected, actual) ?? Equals(expected, actual)))
            throw new Exception($"{method.DeclaringType}.{method.Name}: expected {expected}, got {actual}");
        stateCheck?.Invoke();
        checks++;
    }
    static object New(Type type) => RuntimeHelpers.GetUninitializedObject(type);
    static void Set(object target, string name, object value) => Field(target.GetType(), name).SetValue(target, value);
    static object Get(object target, string name) => Field(target.GetType(), name).GetValue(target);
    static void SameFields(object left, object right, params string[] names)
    {
        foreach (string name in names)
            if (!Equals(Get(left, name), Get(right, name))) throw new Exception($"Field mismatch: {name}");
    }

    static void OpenRa(Assembly original, Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        const string resources = "OpenRA.Mods.Common.Traits.PlayerResources";
        int[] values = { int.MinValue, -1, 0, 1, 9, 10, 11, int.MaxValue };
        foreach (MethodInfo method in methods.Values.Where(m => m.DeclaringType!.FullName == resources))
        {
            foreach (int amount in values)
            foreach (int balance in values)
            foreach (int capacity in new[] { 0, 10, int.MaxValue })
            {
                object a = New(original.GetType(resources)!); object b = New(recovered.GetType(resources)!);
                var fields = new Dictionary<string, int> { ["Cash"] = -7, ["Resources"] = balance, ["ResourceCapacity"] = capacity, ["Earned"] = int.MaxValue - 1, ["Spent"] = -3 };
                foreach (var f in fields)
                {
                    Set(a, f.Key, f.Value);
                    // Only fields actually accessed by the recovered bodies exist in the shell.
                    Set(b, f.Key, f.Value);
                }
                object[] args = method.GetParameters().Length == 0 ? Array.Empty<object>() : new object[] { amount };
                Check(recovered, method, a, b, args, args, () => SameFields(a, b, fields.Keys.ToArray()));
            }
            Check(recovered, method, null, null, method.GetParameters().Length == 0 ? Array.Empty<object>() : new object[] { 1 },
                method.GetParameters().Length == 0 ? Array.Empty<object>() : new object[] { 1 });
        }
        const string ammo = "OpenRA.Mods.Common.Traits.AmmoPool";
        const string info = "OpenRA.Mods.Common.Traits.AmmoPoolInfo";
        foreach (MethodInfo method in methods.Values.Where(m => m.DeclaringType!.FullName == ammo))
        {
            foreach (int count in values)
            foreach (int capacity in values)
            foreach (bool nullInfo in new[] { false, true })
            {
                object a = New(original.GetType(ammo)!); object b = New(recovered.GetType(ammo)!);
                FieldInfo backing = Field(a.GetType(), "<CurrentAmmoCount>k__BackingField");
                backing.SetValue(a, count); Set(b, Alias(backing).TrimStart('@'), count);
                object ai = nullInfo ? null : New(original.GetType(info)!);
                object bi = nullInfo ? null : New(recovered.GetType(info)!);
                if (!nullInfo) { Set(ai, "Ammo", capacity); Set(bi, "Ammo", capacity); }
                Set(a, "Info", ai); Set(b, "Info", bi);
                Check(recovered, method, a, b, Array.Empty<object>(), Array.Empty<object>(), () => {
                    if (!Equals(backing.GetValue(a), Get(b, Alias(backing).TrimStart('@')))) throw new Exception("Ammo mutated");
                    if (!ReferenceEquals(Get(a, "Info"), ai) || !ReferenceEquals(Get(b, "Info"), bi)) throw new Exception("Info identity changed");
                    if (!nullInfo) SameFields(ai, bi, "Ammo");
                });
            }
            Check(recovered, method, null, null, Array.Empty<object>(), Array.Empty<object>());
        }
    }

    static void SameArray(Array left, Array right)
    {
        if (left == null ? right != null : right == null || left.GetType() != right.GetType() ||
            !left.Cast<object>().SequenceEqual(right.Cast<object>())) throw new Exception("Array mismatch");
    }

    static void Arrays(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        foreach (MethodInfo method in methods.Values.Where(m => m.Name.StartsWith("Read", StringComparison.Ordinal) ||
            m.Name.StartsWith("Write", StringComparison.Ordinal)).Where(m => m.Name != "ReadChoice"))
        {
            bool unsigned = method.Name.EndsWith("Unsigned", StringComparison.Ordinal);
            Array[] arrays = unsigned
                ? new Array[] { null, Array.Empty<uint>(), new[] { 0u, 1u, 0x80000000u, uint.MaxValue } }
                : new Array[] { null, Array.Empty<int>(), new[] { int.MinValue, -1, 0, int.MaxValue } };
            object[] domain = method.Name.StartsWith("Write", StringComparison.Ordinal)
                ? unsigned ? new object[] { 0u, 1u, 0x80000000u, uint.MaxValue } : new object[] { int.MinValue, -1, 0, int.MaxValue }
                : new object[] { null };
            foreach (Array values in arrays)
            foreach (int index in new[] { int.MinValue, -1, 0, 1, 3, 4, int.MaxValue })
            foreach (object value in domain)
            {
                Array left = (Array)values?.Clone(), right = (Array)values?.Clone();
                object[] leftArgs = value == null ? new object[] { left, index } : new object[] { left, index, value };
                object[] rightArgs = value == null ? new object[] { right, index } : new object[] { right, index, value };
                Check(recovered, method, null, null, leftArgs, rightArgs,
                    () => SameArray(left, right));
            }
        }
        Type cell = methods.Values.Single(m => m.Name == "OldThenWrite").DeclaringType!;
        Type clone = recovered.GetType(cell.FullName!)!;
        int[] indices = { int.MinValue, -1, 0, 1, 3, 4, int.MaxValue };
        uint[][] states = { null, Array.Empty<uint>(), new[] { 0u, 1u, 0x80000000u, uint.MaxValue } };
        foreach (uint[] state in states)
        foreach (int index in indices)
        foreach (uint value in new[] { 0u, 1u, 0x80000000u, uint.MaxValue })
        {
            object a = New(cell), b = New(clone);
            uint[] left = (uint[])state?.Clone(), right = (uint[])state?.Clone();
            Set(a, "Values", left); Set(b, "Values", right);
            Check(recovered, methods.Values.Single(m => m.Name == "OldThenWrite"), a, b,
                new object[] { index, value }, new object[] { index, value }, () => {
                    SameArray(left, right);
                    if (!ReferenceEquals(Get(a, "Values"), left) || !ReferenceEquals(Get(b, "Values"), right))
                        throw new Exception("Array field identity changed");
                });
        }
        foreach (bool nullReceiver in new[] { false, true })
        foreach (int index in indices)
        {
            object a = nullReceiver ? null : New(cell), b = nullReceiver ? null : New(clone);
            Check(recovered, methods.Values.Single(m => m.Name == "OldThenWrite"), a, b,
                new object[] { index, 1u }, new object[] { index, 1u });
        }
        int[][] signedStates = { null, Array.Empty<int>(), new[] { int.MinValue, -1, 0, int.MaxValue } };
        foreach (int[] state in signedStates)
        foreach (int index in indices)
        {
            object a = New(cell), b = New(clone);
            int[] left = (int[])state?.Clone(), right = (int[])state?.Clone();
            Set(a, "Numbers", left); Set(b, "Numbers", right);
            Set(a, "Effects", 7); Set(b, "Effects", 7);
            Check(recovered, methods.Values.Single(m => m.Name == "EffectfulStore"), a, b,
                new object[] { index }, new object[] { index }, () => {
                    SameArray(left, right); SameFields(a, b, "Effects");
                    if (!ReferenceEquals(Get(a, "Numbers"), left) || !ReferenceEquals(Get(b, "Numbers"), right))
                        throw new Exception("Array field identity changed");
                });
        }
        Check(recovered, methods.Values.Single(m => m.Name == "EffectfulStore"), null, null,
            new object[] { 0 }, new object[] { 0 });
        foreach (bool first in new[] { false, true })
        foreach (bool nullLeft in new[] { false, true })
        foreach (bool nullRight in new[] { false, true })
        {
            uint[] a = nullLeft ? null : new[] { 1u }, b = (uint[])a?.Clone();
            uint[] c = nullRight ? null : new[] { 2u }, d = (uint[])c?.Clone();
            Check(recovered, methods.Values.Single(m => m.Name == "Choose"), null, null,
                new object[] { first, a, c }, new object[] { first, b, d },
                () => { SameArray(a, b); SameArray(c, d); },
                (x, y) => ReferenceEquals(x, first ? a : c) && ReferenceEquals(y, first ? b : d));
        }
        foreach (bool first in new[] { false, true })
        foreach (bool nullLeft in new[] { false, true })
        foreach (bool nullRight in new[] { false, true })
        foreach (int index in indices)
        {
            uint[] a = nullLeft ? null : new[] { 1u, 2u, 3u, 4u }, b = (uint[])a?.Clone();
            uint[] c = nullRight ? null : new[] { uint.MaxValue, 0x80000000u, 0u, 9u }, d = (uint[])c?.Clone();
            Check(recovered, methods.Values.Single(m => m.Name == "ReadChoice"), null, null,
                new object[] { first, a, c, index }, new object[] { first, b, d, index },
                () => { SameArray(a, b); SameArray(c, d); });
        }
        foreach (bool nullTarget in new[] { false, true })
        foreach (bool nullSource in new[] { false, true })
        foreach (bool alias in new[] { false, true })
        foreach (int index in indices)
        {
            uint[] a = nullTarget ? null : new[] { 1u, 2u, 3u, 4u }, b = (uint[])a?.Clone();
            uint[] c = nullSource ? null : alias ? a : new[] { 8u, 9u, 10u, 11u };
            uint[] d = nullSource ? null : alias ? b : (uint[])c.Clone();
            Check(recovered, methods.Values.Single(m => m.Name == "Copy"), null, null,
                new object[] { a, c, index }, new object[] { b, d, index },
                () => { SameArray(a, b); SameArray(c, d); });
        }
        foreach (int length in new[] { -1, 0, 1, 4, 624, 625 })
        {
            uint[] state = length < 0 ? null : new uint[length];
            foreach (string name in new[] { "LengthUnsigned", "WideLength", "CountLoop" })
            {
                uint[] left = (uint[])state?.Clone(), right = (uint[])state?.Clone();
                Check(recovered, methods.Values.Single(m => m.Name == name), null, null,
                    new object[] { left }, new object[] { right }, () => SameArray(left, right));
            }
            int[] sl = length < 0 ? null : new int[length], sr = (int[])sl?.Clone();
            Check(recovered, methods.Values.Single(m => m.Name == "LengthSigned"), null, null,
                new object[] { sl }, new object[] { sr }, () => SameArray(sl, sr));
            foreach (int initial in new[] { 7, int.MaxValue })
            {
                object a = New(cell), b = New(clone);
                uint[] left = (uint[])state?.Clone(), right = (uint[])state?.Clone();
                Set(a, "Values", left); Set(b, "Values", right);
                Set(a, "Effects", initial); Set(b, "Effects", initial);
                Check(recovered, methods.Values.Single(m => m.Name == "LengthAfterEffect"), a, b,
                    Array.Empty<object>(), Array.Empty<object>(), () => {
                        SameFields(a, b, "Effects"); SameArray(left, right);
                        if (!ReferenceEquals(Get(a, "Values"), left) || !ReferenceEquals(Get(b, "Values"), right))
                            throw new Exception("Length read changed array identity");
                    });
            }
            foreach (bool nullReplacement in new[] { false, true })
            foreach (bool alias in new[] { false, true })
            {
                object a = New(cell), b = New(clone);
                uint[] left = (uint[])state?.Clone(), right = (uint[])state?.Clone();
                uint[] al = nullReplacement ? null : alias ? left : new uint[3];
                uint[] ar = nullReplacement ? null : alias ? right : new uint[3];
                Set(a, "Values", left); Set(b, "Values", right);
                Check(recovered, methods.Values.Single(m => m.Name == "LengthSnapshot"), a, b,
                    new object[] { al }, new object[] { ar }, () => {
                        SameArray(left, right); SameArray(al, ar);
                        if (!ReferenceEquals(Get(a, "Values"), al) || !ReferenceEquals(Get(b, "Values"), ar))
                            throw new Exception("Length snapshot lost prior field reassignment");
                    });
            }
        }
        Check(recovered, methods.Values.Single(m => m.Name == "LengthAfterEffect"), null, null,
            Array.Empty<object>(), Array.Empty<object>());
        Check(recovered, methods.Values.Single(m => m.Name == "LengthSnapshot"), null, null,
            new object[] { null }, new object[] { null });
        object ba = New(cell), bb = New(clone);
        Set(ba, "Effects", int.MaxValue); Set(bb, "Effects", int.MaxValue);
        MethodInfo bump = methods.Values.Single(m => m.Name == "Bump");
        Check(recovered, bump, ba, bb, Array.Empty<object>(), Array.Empty<object>(), () => SameFields(ba, bb, "Effects"));
        Check(recovered, bump, null, null, Array.Empty<object>(), Array.Empty<object>());
        Check(recovered, methods.Values.Single(m => m.Name == "Null"), null, null, Array.Empty<object>(), Array.Empty<object>());
        if (methods.Values.Select(m => m.Name).Order().SequenceEqual(new[] {
            "ReadSigned", "ReadUnsigned", "WriteSigned", "WriteUnsigned", "Null", "OldThenWrite", "Choose", "Bump", "EffectfulStore", "Copy", "ReadChoice",
            "LengthSigned", "LengthUnsigned", "WideLength", "LengthAfterEffect", "LengthSnapshot", "CountLoop"
        }.Order()) == false) throw new Exception("Array method selection drift");
    }

    static void Addresses(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        Type cell = methods.Values.First().DeclaringType!;
        Type clone = recovered.GetType(cell.FullName!)!;
        int[] indices = { int.MinValue, -1, 0, 1, 2, 3, int.MaxValue };
        foreach (int length in new[] { -1, 0, 1, 3 })
        foreach (int index in indices)
        {
            bool valid = length >= 0 && index >= 0 && index < length;
            foreach (string name in new[] { "Take", "TakeSigned", "Duplicate" })
            {
                object a = New(cell), b = New(clone);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                Array left = length < 0 ? null : name == "TakeSigned"
                    ? new[] { int.MinValue, -1, int.MaxValue }.Take(length).ToArray()
                    : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                Array right = (Array)left?.Clone();
                Check(recovered, methods.Values.Single(m => m.Name == name), a, b,
                    new object[] { left, index }, new object[] { right, index }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right);
                        if ((int)Get(a, "Effects") != (valid ? 2 : 1))
                            throw new Exception("Address acquisition or intervening call changed effect timing");
                    });
            }
            foreach (bool nullReplacement in new[] { false, true })
            foreach (bool alias in new[] { false, true })
            {
                object a = New(cell), b = New(clone);
                uint[] left = length < 0 ? null : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                uint[] right = (uint[])left?.Clone();
                uint[] al = nullReplacement ? null : alias ? left : new uint[2];
                uint[] ar = nullReplacement ? null : alias ? right : new uint[2];
                Set(a, "Values", left); Set(b, "Values", right);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                Check(recovered, methods.Values.Single(m => m.Name == "Retarget"), a, b,
                    new object[] { al, index }, new object[] { ar, index }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right); SameArray(al, ar);
                        if ((int)Get(a, "Effects") != (valid ? 2 : 1)
                            || !ReferenceEquals(Get(a, "Values"), valid ? al : left)
                            || !ReferenceEquals(Get(b, "Values"), valid ? ar : right))
                            throw new Exception("Address checks were delayed past field reassignment");
                    });
            }
        }
        foreach (int length in new[] { -1, 0, 1, 3 })
        foreach (int index in indices)
        {
            foreach (string name in new[] { "ReadUnsigned", "ReadSignedBits", "ReadSnapshot", "ReadAfterWrite" })
            foreach (uint replacement in new[] { 0u, 1u, 0x80000000u, uint.MaxValue })
            {
                object a = New(cell), b = New(clone);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                Array left = length < 0 ? null : name == "ReadSignedBits"
                    ? new[] { int.MinValue, -1, int.MaxValue }.Take(length).ToArray()
                    : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                Array right = (Array)left?.Clone();
                bool writes = name == "ReadSnapshot" || name == "ReadAfterWrite";
                object[] la = writes ? new object[] { left, index, replacement } : new object[] { left, index };
                object[] ra = writes ? new object[] { right, index, replacement } : new object[] { right, index };
                Check(recovered, methods.Values.Single(m => m.Name == name), a, b, la, ra,
                    () => { SameFields(a, b, "Effects"); SameArray(left, right); });
            }
            foreach (bool nullReplacement in new[] { false, true })
            foreach (bool alias in new[] { false, true })
            {
                object a = New(cell), b = New(clone);
                uint[] left = length < 0 ? null : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                uint[] right = (uint[])left?.Clone();
                uint[] al = nullReplacement ? null : alias ? left : new uint[2];
                uint[] ar = nullReplacement ? null : alias ? right : new uint[2];
                Set(a, "Values", left); Set(b, "Values", right);
                Check(recovered, methods.Values.Single(m => m.Name == "ReadAfterRetarget"), a, b,
                    new object[] { al, index }, new object[] { ar, index }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right); SameArray(al, ar);
                        bool valid = length >= 0 && index >= 0 && index < length;
                        if (!ReferenceEquals(Get(a, "Values"), valid ? al : left)
                            || !ReferenceEquals(Get(b, "Values"), valid ? ar : right))
                            throw new Exception("Indirect read changed the addressed array identity");
                    });
            }
        }
        foreach (string name in new[] { "Take", "TakeSigned", "Duplicate", "Retarget", "ReadUnsigned", "ReadSignedBits", "ReadAfterRetarget", "ReadSnapshot", "ReadAfterWrite" })
        {
            object[] values = name == "ReadSnapshot" || name == "ReadAfterWrite"
                ? new object[] { null, 0, 1u } : new object[] { null, 0 };
            Check(recovered, methods.Values.Single(m => m.Name == name), null, null, values, values);
        }
        object ba = New(cell), bb = New(clone);
        Set(ba, "Effects", int.MaxValue); Set(bb, "Effects", int.MaxValue);
        Check(recovered, methods.Values.Single(m => m.Name == "Bump"), ba, bb,
            Array.Empty<object>(), Array.Empty<object>(), () => SameFields(ba, bb, "Effects"));
        Check(recovered, methods.Values.Single(m => m.Name == "Bump"), null, null,
            Array.Empty<object>(), Array.Empty<object>());
        if (!methods.Values.Select(m => m.Name).Order().SequenceEqual(new[] {
            "Take", "TakeSigned", "Duplicate", "Retarget", "Bump", "ReadUnsigned", "ReadSignedBits", "ReadSnapshot", "ReadAfterWrite", "ReadAfterRetarget"
        }.Order())) throw new Exception("Address method selection drift");
    }

    static void Conversions(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        long[] wide = { long.MinValue, -4294967297L, -2147483649L, -1L, 0L, 1L,
            2147483647L, 2147483648L, 4294967295L, 4294967296L, long.MaxValue };
        object[] Domain(Type type)
        {
            if (type == typeof(int)) return new object[] { int.MinValue, -1, 0, 1, int.MaxValue };
            if (type == typeof(uint)) return new object[] { 0u, 1u, 0x7fffffffu, 0x80000000u, uint.MaxValue };
            if (type == typeof(long)) return wide.Cast<object>().ToArray();
            if (type == typeof(ulong)) return new object[] { 0UL, 1UL, 2147483648UL, 4294967295UL,
                4294967296UL, 0x8000000000000000UL, ulong.MaxValue };
            if (type == typeof(nint)) return wide.Select(n => (object)unchecked((nint)n)).ToArray();
            if (type == typeof(nuint)) return new ulong[] { 0UL, 1UL, 2147483648UL, 4294967295UL,
                4294967296UL, 0x8000000000000000UL, ulong.MaxValue }.Select(n => (object)unchecked((nuint)n)).ToArray();
            throw new Exception($"Missing conversion test domain: {type}");
        }
        foreach (MethodInfo method in methods.Values)
        {
            if (method.IsStatic)
            {
                var parameters = method.GetParameters();
                void Enumerate(int index, object[] values)
                {
                    if (index == values.Length)
                    {
                        Check(recovered, method, null, null, values, values);
                        return;
                    }
                    foreach (object value in Domain(parameters[index].ParameterType))
                    {
                        values[index] = value;
                        Enumerate(index + 1, values);
                    }
                }
                Enumerate(0, new object[parameters.Length]);
            }
            else
            {
                if (method.Name != "ReadThenWrite") throw new Exception("Unexpected conversion instance method");
                foreach (long initial in wide)
                foreach (long replacement in wide)
                {
                    object a = New(method.DeclaringType!), b = New(recovered.GetType(method.DeclaringType!.FullName!)!);
                    Set(a, "Wide", initial); Set(b, "Wide", initial);
                    Check(recovered, method, a, b, new object[] { replacement }, new object[] { replacement },
                        () => SameFields(a, b, "Wide"));
                }
                Check(recovered, method, null, null, new object[] { 1L }, new object[] { 1L });
            }
        }
        if (methods.Count != 16) throw new Exception("Conversion method selection drift");
        Console.WriteLine($"Conversion execution width: {IntPtr.Size * 8} bits; no cross-width execution claim.");
    }

    static void Native(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        object[] signed = IntPtr.Size == 8
            ? new object[] { (nint)0, (nint)1, (nint)(-1), unchecked((nint)0x80000000L), unchecked((nint)long.MinValue), unchecked((nint)long.MaxValue) }
            : new object[] { (nint)0, (nint)1, (nint)(-1), (nint)int.MinValue, (nint)int.MaxValue };
        object[] unsigned = IntPtr.Size == 8
            ? new object[] { (nuint)0, (nuint)1, unchecked((nuint)0x100000000UL), unchecked((nuint)0x8000000000000000UL), unchecked((nuint)ulong.MaxValue) }
            : new object[] { (nuint)0, (nuint)1, unchecked((nuint)0x80000000U), (nuint)uint.MaxValue };
        foreach (MethodInfo method in methods.Values)
        {
            bool isUnsigned = method.ReturnType == typeof(nuint);
            object[] domain = isUnsigned ? unsigned : signed;
            if (method.Name == "Choose")
            {
                foreach (bool first in new[] { false, true })
                foreach (object left in signed)
                foreach (object right in signed)
                    Check(recovered, method, null, null, new[] { (object)first, left, right }, new[] { (object)first, left, right });
            }
            else if (!method.IsStatic)
            {
                string field = isUnsigned ? "Unsigned" : "Signed";
                foreach (object initial in domain)
                foreach (object value in domain)
                {
                    object a = New(method.DeclaringType!), b = New(recovered.GetType(method.DeclaringType!.FullName!)!);
                    Set(a, field, initial); Set(b, field, initial);
                    Check(recovered, method, a, b, new[] { value }, new[] { value }, () => SameFields(a, b, field));
                }
                foreach (object value in domain)
                    Check(recovered, method, null, null, new[] { value }, new[] { value });
            }
            else if (method.Name == "ReplaceStatic")
            {
                FieldInfo a = Field(method.DeclaringType!, "Slot"), b = Field(recovered.GetType(method.DeclaringType!.FullName!)!, "Slot");
                foreach (object initial in signed)
                foreach (object value in signed)
                {
                    a.SetValue(null, initial); b.SetValue(null, initial);
                    Check(recovered, method, null, null, new[] { value }, new[] { value }, () => {
                        if (!Equals(a.GetValue(null), b.GetValue(null))) throw new Exception("Native static field mismatch");
                    });
                }
            }
            else
            {
                if (method.Name is not ("Identity" or "UnsignedIdentity" or "Forward")) throw new Exception("Unexpected native method");
                foreach (object value in domain)
                    Check(recovered, method, null, null, new[] { value }, new[] { value });
            }
        }
        if (methods.Count != 7) throw new Exception("Native method selection drift");
        Console.WriteLine($"Native storage execution width: {IntPtr.Size * 8} bits; no cross-width execution claim.");
    }

    static void References(Assembly original, Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        Type cell = original.GetType("ReferenceCases.Cell")!;
        Type clone = recovered.GetType(cell.FullName!)!;
        int[] values = { int.MinValue, -1, 0, 1, int.MaxValue };
        foreach (MethodInfo method in methods.Values.Where(m => !m.IsStatic))
        {
            foreach (int initial in values)
            foreach (int delta in values)
            foreach (bool nullOther in new[] { false, true })
            {
                object a = New(method.DeclaringType!); object b = New(recovered.GetType(method.DeclaringType!.FullName!)!);
                if (cell.IsAssignableFrom(method.DeclaringType))
                {
                    Set(a, "Value", initial); Set(b, "Value", initial);
                    Set(a, "Other", nullOther ? null : a); Set(b, "Other", nullOther ? null : b);
                    FieldInfo backing = Field(cell, "<Count>k__BackingField");
                    backing.SetValue(a, initial); Set(b, Alias(backing).TrimStart('@'), initial);
                }
                else { Set(a, "Number", initial); Set(b, "Number", initial); }
                object[] args = method.GetParameters().Length == 0 ? Array.Empty<object>() : new object[] { delta };
                Check(recovered, method, a, b, args, args, () => {
                    if (cell.IsAssignableFrom(method.DeclaringType)) {
                        SameFields(a, b, "Value");
                        FieldInfo f = Field(cell, "<Count>k__BackingField");
                        if (!Equals(f.GetValue(a), Get(b, Alias(f).TrimStart('@')))) throw new Exception("Count differs");
                    } else SameFields(a, b, "Number");
                });
            }
            object[] nilArgs = method.GetParameters().Length == 0 ? Array.Empty<object>() : new object[] { 1 };
            Check(recovered, method, null, null, nilArgs, nilArgs);
        }
        MethodInfo store = methods.Values.Single(m => m.Name == "StoreAfterRead");
        foreach (bool nullTarget in new[] { false, true })
        foreach (bool nullSource in new[] { false, true })
        foreach (bool alias in new[] { false, true })
        {
            object a = New(cell), b = New(clone), c = alias ? a : New(cell), d = alias ? b : New(clone);
            Set(a, "Value", 3); Set(b, "Value", 3); Set(c, "Value", 17); Set(d, "Value", 17);
            Check(recovered, store, null, null, new[] { nullTarget ? null : a, nullSource ? null : c },
                new[] { nullTarget ? null : b, nullSource ? null : d }, () => { SameFields(a, b, "Value"); SameFields(c, d, "Value"); });
        }
        MethodInfo clear = methods.Values.Single(m => m.Name == "ClearOther");
        object ca = New(cell), cb = New(clone);
        Set(ca, "Other", ca); Set(cb, "Other", cb);
        Check(recovered, clear, null, null, new[] { ca }, new[] { cb },
            () => SameFields(ca, cb, "Other"), (x, y) => ReferenceEquals(x, ca) && ReferenceEquals(y, cb));
        MethodInfo nil = methods.Values.Single(m => m.Name == "Null");
        Check(recovered, nil, null, null, Array.Empty<object>(), Array.Empty<object>());
        MethodInfo choose = methods.Values.Single(m => m.Name == "Choose");
        foreach (bool first in new[] { false, true })
        {
            object a = New(cell), b = New(clone), c = New(cell), d = New(clone);
            Check(recovered, choose, null, null, new[] { (object)first, a, c }, new[] { (object)first, b, d },
                equal: (x, y) => ReferenceEquals(x, first ? a : c) && ReferenceEquals(y, first ? b : d));
        }
    }
}
