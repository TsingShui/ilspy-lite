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
        var members = job.Methods.ToDictionary(e => e.Token, e => original.ManifestModule.ResolveMethod(e.Token)!);
        if (args[0] == "prepare")
        {
            Prepare(job, original, members, args[2]);
            Console.WriteLine($"Prepared {members.Count} recovered bodies; no original method executed.");
            return;
        }
        if (args[0] != "run") throw new Exception("Unknown mode");
        Assembly recovered = new AssemblyLoadContext("recovered", isCollectible: true).LoadFromAssemblyPath(args[2]);
        if (job.Kind == "default")
        {
            DefaultConstructor(recovered, members);
            Console.WriteLine($"default: {checks} return/exception/state comparisons passed");
            return;
        }
        if (job.Kind == "random-float")
        {
            RandomFloat(recovered, members);
            Console.WriteLine($"random-float: {checks} return/exception/state comparisons passed");
            return;
        }
        if (job.Kind == "random-ranges")
        {
            RandomRanges(recovered, members);
            Console.WriteLine($"random-ranges: {checks} return/exception/state comparisons passed");
            return;
        }
        if (job.Kind == "random-seed")
        {
            RandomSeed(recovered, members);
            Console.WriteLine($"random-seed: {checks} return/exception/state comparisons passed");
            return;
        }
        if (job.Kind == "constructors")
        {
            Constructors(recovered, members);
            Console.WriteLine($"constructors: {checks} return/exception/state comparisons passed");
            return;
        }
        var methods = members.ToDictionary(p => p.Key, p => (MethodInfo)p.Value);
        if (job.Kind == "floats") Floats(recovered, methods);
        else if (job.Kind == "exceptions") Exceptions(recovered, methods);
        else if (job.Kind == "strings") Strings(recovered, methods);
        else if (job.Kind == "clock") Clock(recovered, methods);
        else if (job.Kind == "random-state") RandomState(recovered, methods);
        else if (job.Kind == "openra") OpenRa(original, recovered, methods);
        else if (job.Kind == "arrays") Arrays(recovered, methods);
        else if (job.Kind == "native") Native(recovered, methods);
        else if (job.Kind == "conversions") Conversions(recovered, methods);
        else if (job.Kind == "addresses") Addresses(recovered, methods);
        else if (job.Kind == "references") References(original, recovered, methods);
        else throw new Exception("Unknown corpus kind");
        Console.WriteLine($"{job.Kind}: {checks} return/exception/state comparisons passed");
    }

    static Type ReturnType(MethodBase method) => method is MethodInfo info ? info.ReturnType : typeof(void);

    static void Prepare(Job job, Assembly original, Dictionary<int, MethodBase> methods, string output)
    {
        var fields = new Dictionary<int, FieldInfo>();
        var types = new HashSet<Type>();
        var code = new List<string>();
        var coreConstructors = new HashSet<int>();
        var clocks = new HashSet<int>();
        var strings = new HashSet<int>();
        var exceptions = new HashSet<int>();
        var absoluteCalls = new HashSet<int>();
        bool hasFloat = false;
        foreach (Entry entry in job.Methods)
        {
            MethodBase method = methods[entry.Token];
            types.Add(method.DeclaringType!);
            string source = File.ReadAllText(entry.Source);
            hasFloat |= source.Contains("__cil_F", StringComparison.Ordinal);
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
            foreach (Match match in Regex.Matches(source, @"__cil_ctor_call_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                if (!methods.TryGetValue(token, out MethodBase target) || target is not ConstructorInfo)
                    throw new Exception("A delegated constructor dependency was not recovered");
            }
            foreach (Match match in Regex.Matches(source, @"__cil_core_object_ctor_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                MethodBase target = original.ManifestModule.ResolveMethod(token)!;
                if (target is not ConstructorInfo || target.DeclaringType != typeof(object)
                    || target.IsStatic || target.GetParameters().Length != 0)
                    throw new Exception("Core constructor intrinsic target mismatch");
                coreConstructors.Add(token);
            }
            foreach (Match match in Regex.Matches(source, @"__cil_tick_count_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                MethodBase target = original.ManifestModule.ResolveMethod(token)!;
                if (target is not MethodInfo clock || clock.DeclaringType != typeof(Environment)
                    || clock.Name != "get_TickCount" || !clock.IsStatic
                    || clock.ReturnType != typeof(int) || clock.GetParameters().Length != 0)
                    throw new Exception("Clock intrinsic target mismatch");
                clocks.Add(token);
            }
            foreach (Match match in Regex.Matches(source, @"__cil_ldstr_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                if ((uint)token >> 24 != 0x70) throw new Exception("Invalid string literal token");
                original.ManifestModule.ResolveString(token); // Validate original identity without executing a selected body.
                strings.Add(token);
            }
            foreach (Match match in Regex.Matches(source, @"__cil_newobj_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                MethodBase target = original.ManifestModule.ResolveMethod(token)!;
                if (target is not ConstructorInfo || target.DeclaringType != typeof(ArgumentOutOfRangeException)
                    || target.IsStatic || !target.GetParameters().Select(p => p.ParameterType)
                        .SequenceEqual(new[] { typeof(string), typeof(string) }))
                    throw new Exception("Exception allocation target mismatch");
                exceptions.Add(token);
            }
            foreach (Match match in Regex.Matches(source, @"__cil_abs_r4_([0-9A-F]{8})\("))
            {
                int token = Convert.ToInt32(match.Groups[1].Value, 16);
                MethodBase target = original.ManifestModule.ResolveMethod(token)!;
                if (target is not MethodInfo absolute || absolute.DeclaringType != typeof(Math)
                    || absolute.Name != "Abs" || !absolute.IsStatic || absolute.ReturnType != typeof(float)
                    || !absolute.GetParameters().Select(p => p.ParameterType).SequenceEqual(new[] { typeof(float) }))
                    throw new Exception("Float absolute-call target mismatch");
                absoluteCalls.Add(token);
            }
            string declaration = method is ConstructorInfo ? $"__cil_ctor_body_{entry.Token:X8}(" : "@" + method.Name + "(";
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
            foreach (MethodInfo method in methods.Values.OfType<MethodInfo>().Where(m => m.IsStatic && m.DeclaringType == type))
            {
                string parameters = string.Join(", ", method.GetParameters().Select((p, i) => TypeName(p.ParameterType) + " a" + i));
                string arguments = string.Join(", ", method.GetParameters().Select((p, i) => "a" + i));
                members += $"\npublic static {TypeName(method.ReturnType)} @{method.Name}({parameters}) => global::Recovered.M_{method.MetadataToken:X8}({arguments});";
            }
            shells.Add($"namespace {type.Namespace} {{ public class {type.Name}{parent} {{ {members} }} }}");
        }
        if (methods.Values.Any(m => m is ConstructorInfo))
            code.Add("public static global::System.Action<int, object, object[]> ConstructorCallObserver;");
        foreach (var pair in methods)
        {
            MethodBase method = pair.Value;
            if (method.IsStatic) continue;
            var parameters = new[] { TypeName(method.DeclaringType!) + " self" }
                .Concat(method.GetParameters().Select((p, i) => TypeName(p.ParameterType) + " a" + i));
            string arguments = string.Join(", ", new[] { "self" }.Concat(method.GetParameters().Select((p, i) => "a" + i)));
            if (method is ConstructorInfo)
            {
                string observedArgs = string.Join(", ", method.GetParameters().Select((p, i) => "a" + i));
                code.Add($"public static void __cil_ctor_call_{pair.Key:X8}({string.Join(", ", parameters)}) {{ ConstructorCallObserver?.Invoke({pair.Key}, self, new object[] {{ {observedArgs} }}); M_{pair.Key:X8}({arguments}); }}");
            }
            else
                code.Add($"public static {TypeName(ReturnType(method))} __cil_direct_{pair.Key:X8}({string.Join(", ", parameters)}) => M_{pair.Key:X8}({arguments});");
        }
        if (coreConstructors.Count != 0)
        {
            code.Add("public static global::System.Action<object> CoreConstructorCall;");
            foreach (int token in coreConstructors)
                code.Add($"public static void __cil_core_object_ctor_{token:X8}(object self) => CoreConstructorCall(self);");
        }
        if (clocks.Count != 0)
        {
            code.Add("public static global::System.Func<int> TickCount;");
            foreach (int token in clocks)
                code.Add($"public static int __cil_tick_count_{token:X8}() => TickCount();");
        }
        if (strings.Count != 0)
        {
            code.Add("public static global::System.Action<int, string> StringLoadObserver;");
            foreach (int token in strings)
                code.Add($"public static string __cil_ldstr_{token:X8}(string units) {{ StringLoadObserver?.Invoke({token}, units); return units; }}");
        }
        if (exceptions.Count != 0)
        {
            code.Add("public static global::System.Action<int, string, string> ExceptionAllocationObserver;");
            foreach (int token in exceptions)
                code.Add($"public static global::System.ArgumentOutOfRangeException __cil_newobj_{token:X8}(string parameter, string message) {{ ExceptionAllocationObserver?.Invoke({token}, parameter, message); return new global::System.ArgumentOutOfRangeException(parameter, message); }}");
        }
        if (absoluteCalls.Count != 0)
        {
            code.Add("public static global::System.Action<int, float> AbsoluteCallObserver;");
            foreach (int token in absoluteCalls)
                code.Add($"public static float __cil_abs_r4_{token:X8}(float value) {{ AbsoluteCallObserver?.Invoke({token}, value); return global::System.Math.Abs(value); }}");
        }
        string floatImports = hasFloat ? "using __cil_F = System.Single;\nusing static CilFloat;\n" : "";
        File.WriteAllText(output, floatImports + string.Join("\n", shells) + "\npublic static class Recovered {\n" + string.Join("\n", code) + "\n}\n");
    }

    static object Original(MethodBase method, object receiver, object[] args)
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
            if (method is ConstructorInfo constructor) il.Emit(OpCodes.Call, constructor);
            else il.Emit(OpCodes.Call, (MethodInfo)method);
            Type returnType = ReturnType(method);
            if (returnType == typeof(void)) il.Emit(OpCodes.Ldnull);
            else if (returnType.IsValueType) il.Emit(OpCodes.Box, returnType);
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
    record Fault(string Type, string Parameter, string Message, int HResult, object ActualValue);
    static object DetailedOutcome(Func<object> action)
    {
        try { return action(); }
        catch (Exception e)
        {
            Exception error = e is TargetInvocationException t ? t.InnerException! : e;
            var range = error as ArgumentOutOfRangeException;
            return new Fault(error.GetType().FullName, (error as ArgumentException)?.ParamName,
                error.Message, error.HResult, range?.ActualValue);
        }
    }
    static void Check(Assembly recovered, MethodBase method, object left, object right, object[] leftArgs, object[] rightArgs,
        Action stateCheck = null, Func<object, object, bool> equal = null, bool detailedExceptions = false)
    {
        Func<Func<object>, object> capture = detailedExceptions ? DetailedOutcome : Outcome;
        object expected = capture(() => Original(method, left, leftArgs));
        object actual = capture(() => recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!
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

    static Action<object> CoreObjectCall()
    {
        var dm = new DynamicMethod("ExactCoreObjectConstructor", typeof(void), new[] { typeof(object) }, typeof(object).Module, true);
        ILGenerator il = dm.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Call, typeof(object).GetConstructor(Type.EmptyTypes)!);
        il.Emit(OpCodes.Ret);
        return (Action<object>)dm.CreateDelegate(typeof(Action<object>));
    }

    static bool SameFloatBits(object expected, object actual) => expected is float left && actual is float right
        ? BitConverter.SingleToInt32Bits(left) == BitConverter.SingleToInt32Bits(right) : Equals(expected, actual);

    static void Floats(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        string[] names = { "FromI4", "FromU4Bits", "FromI8", "FromU8Bits", "FromNative", "FromUNativeBits", "Identity", "Choose", "Replace", "ConvertedSnapshot",
            "Unit", "NegativeZero", "Largest", "Infinite", "NotANumber", "Smallest", "Divide", "Scale", "DivideSnapshot", "Absolute", "AbsoluteSnapshot" };
        if (methods.Count != names.Length || !methods.Values.Select(m => m.Name).Order().SequenceEqual(names.Order()))
            throw new Exception("Float conversion selection drift");
        int[] i4 = { int.MinValue, -16777219, -16777217, -1, 0, 1, 16777215, 16777216, 16777217, 16777219, int.MaxValue };
        long[] i8 = { long.MinValue, -9007199254740993L, -16777217, -1, 0, 1, 16777217, 9007199254740993L, long.MaxValue };
        uint[] floatBits = { 0u, 0x80000000u, 1u, 0x007fffffu, 0x00800000u, 0x3f800000u, 0xbf800000u,
            0x40400000u, 0x3eaaaaabu, 0x4f000000u,
            0x7f7fffffu, 0xff7fffffu, 0x7f800000u, 0xff800000u, 0x7fc01234u, 0xffc05678u };
        float[] floats = floatBits.Select(n => BitConverter.Int32BitsToSingle(unchecked((int)n))).ToArray();
        object[] Domain(Type type)
        {
            if (type == typeof(int)) return i4.Cast<object>().ToArray();
            if (type == typeof(uint)) return new[] { 0u, 1u, 16777217u, 0x80000000u, uint.MaxValue }.Cast<object>().ToArray();
            if (type == typeof(long)) return i8.Cast<object>().ToArray();
            if (type == typeof(ulong)) return new[] { 0UL, 1UL, 16777217UL, 0x8000000000000000UL, ulong.MaxValue }.Cast<object>().ToArray();
            if (type == typeof(nint)) return i8.Select(n => (object)unchecked((nint)n)).ToArray();
            if (type == typeof(nuint)) return new[] { 0UL, 1UL, 16777217UL, 0x8000000000000000UL, ulong.MaxValue }.Select(n => (object)unchecked((nuint)n)).ToArray();
            if (type == typeof(float)) return floats.Cast<object>().ToArray();
            throw new Exception("Missing float-conversion domain");
        }
        foreach (MethodInfo method in methods.Values.Where(m => m.IsStatic && m.GetParameters().Length == 1))
            foreach (object value in Domain(method.GetParameters().Single().ParameterType))
                Check(recovered, method, null, null, new[] { value }, new[] { value }, equal: SameFloatBits);
        MethodInfo choose = methods.Values.Single(m => m.Name == "Choose");
        foreach (bool first in new[] { false, true })
        foreach (float left in floats)
        foreach (float right in floats)
            Check(recovered, choose, null, null, new object[] { first, left, right }, new object[] { first, left, right }, equal: SameFloatBits);
        MethodInfo replace = methods.Values.Single(m => m.Name == "Replace");
        Type cell = replace.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        foreach (float initial in floats)
        foreach (float replacement in floats)
        {
            object a = New(cell), b = New(clone);
            Set(a, "Number", initial); Set(b, "Number", initial);
            Check(recovered, replace, a, b, new object[] { replacement }, new object[] { replacement }, () => {
                if (!SameFloatBits(Get(a, "Number"), Get(b, "Number"))) throw new Exception("Float storage bits changed");
            }, SameFloatBits);
        }
        foreach (var constant in new[] { (Name: "Unit", Bits: 0x3f800000u), (Name: "NegativeZero", Bits: 0x80000000u),
            (Name: "Largest", Bits: 0x7f7fffffu), (Name: "Infinite", Bits: 0x7f800000u),
            (Name: "NotANumber", Bits: 0xffc00000u), (Name: "Smallest", Bits: 1u) })
        {
            MethodInfo method = methods.Values.Single(m => m.Name == constant.Name);
            Check(recovered, method, null, null, Array.Empty<object>(), Array.Empty<object>(), equal: SameFloatBits);
            object actual = recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!.Invoke(null, Array.Empty<object>());
            if (actual is not float value || unchecked((uint)BitConverter.SingleToInt32Bits(value)) != constant.Bits)
                throw new Exception("Float constant bits were not retained");
        }
        MethodInfo divide = methods.Values.Single(m => m.Name == "Divide");
        MethodInfo divideSnapshot = methods.Values.Single(m => m.Name == "DivideSnapshot");
        foreach (float numerator in floats)
        foreach (float denominator in floats)
        {
            Check(recovered, divide, null, null, new object[] { numerator, denominator }, new object[] { numerator, denominator }, equal: SameFloatBits);
            foreach (float replacement in new[] { 0f, -1f })
            {
                object a = New(cell), b = New(clone);
                Set(a, "Number", numerator); Set(b, "Number", numerator);
                Check(recovered, divideSnapshot, a, b, new object[] { denominator, replacement }, new object[] { denominator, replacement }, () => {
                    if (!SameFloatBits(Get(a, "Number"), Get(b, "Number"))) throw new Exception("Division snapshot lost following store");
                }, SameFloatBits);
            }
        }
        Check(recovered, divideSnapshot, null, null, new object[] { 0f, 1f }, new object[] { 0f, 1f });
        MethodInfo snapshot = methods.Values.Single(m => m.Name == "ConvertedSnapshot");
        foreach (int initial in i4)
        foreach (int replacement in i4)
        {
            object a = New(cell), b = New(clone);
            Set(a, "Count", initial); Set(b, "Count", initial);
            Check(recovered, snapshot, a, b, new object[] { replacement }, new object[] { replacement },
                () => SameFields(a, b, "Count"), SameFloatBits);
        }
        foreach (var pair in new[] { (Value: 16777217, Bits: 0x4b800000), (Value: 16777219, Bits: 0x4b800002),
            (Value: int.MaxValue, Bits: 0x4f000000), (Value: int.MinValue, Bits: unchecked((int)0xcf000000u)) })
        {
            object value = recovered.GetType("Recovered")!.GetMethod($"M_{methods.Values.Single(m => m.Name == "FromI4").MetadataToken:X8}")!
                .Invoke(null, new object[] { pair.Value });
            if (value is not float result || BitConverter.SingleToInt32Bits(result) != pair.Bits)
                throw new Exception("Explicit binary32 rounding boundary mismatch");
        }
        Check(recovered, replace, null, null, new object[] { 1f }, new object[] { 1f });
        Check(recovered, snapshot, null, null, new object[] { 1 }, new object[] { 1 });
        MethodInfo absoluteSnapshot = methods.Values.Single(m => m.Name == "AbsoluteSnapshot");
        foreach (float initial in floats)
        foreach (float replacement in floats)
        {
            object a = New(cell), b = New(clone);
            Set(a, "Number", initial); Set(b, "Number", initial);
            int calls = 0;
            recovered.GetType("Recovered")!.GetField("AbsoluteCallObserver")!.SetValue(null, (Action<int, float>)((token, value) => {
                if (!SameFloatBits(value, initial) || !SameFloatBits(Get(b, "Number"), replacement))
                    throw new Exception("Absolute call moved across field mutation or lost old bits");
                calls++;
            }));
            Check(recovered, absoluteSnapshot, a, b, new object[] { replacement }, new object[] { replacement }, () => {
                if (calls != 1 || !SameFloatBits(Get(a, "Number"), Get(b, "Number")))
                    throw new Exception("Absolute call missing or duplicated");
            }, SameFloatBits);
        }
        recovered.GetType("Recovered")!.GetField("AbsoluteCallObserver")!.SetValue(null, null);
        Check(recovered, absoluteSnapshot, null, null, new object[] { 1f }, new object[] { 1f });
        Console.WriteLine($"Floats: bitwise binary32 conversion/constants/division, IEEE special values and snapshots; local .NET runtime carrier, native width {IntPtr.Size * 8}, no cross-runtime F-precision claim.");
    }

    static void Exceptions(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        string[] names = { "Make", "Literal", "Null", "Build", "Discard", "Raise", "ThrowExisting", "ThrowNull" };
        if (methods.Count != names.Length || !methods.Values.Select(m => m.Name).Order().SequenceEqual(names.Order()))
            throw new Exception("Exception object selection drift");
        bool EqualObject(object x, object y)
        {
            if (x == null || y == null) return x == null && y == null;
            if (x is not ArgumentOutOfRangeException left || y is not ArgumentOutOfRangeException right
                || ReferenceEquals(left, right)) return false;
            return left.GetType() == right.GetType() && left.ParamName == right.ParamName
                && left.Message == right.Message && left.HResult == right.HResult
                && left.ActualValue == null && right.ActualValue == null
                && left.InnerException == null && right.InnerException == null;
        }
        MethodInfo make = methods.Values.Single(m => m.Name == "Make");
        string[] values = { null, "", "parameter", "message", "\ud800\0\udfff", new string(new[] { 'p', 'a', 'r', 'a', 'm' }) };
        foreach (string parameter in values)
        foreach (string message in values)
        {
            Check(recovered, make, null, null, new object[] { parameter, message }, new object[] { parameter, message }, equal: EqualObject);
            foreach (string name in new[] { "Build", "Discard" })
            foreach (bool nullReceiver in new[] { false, true })
            {
                MethodInfo method = methods.Values.Single(m => m.Name == name);
                object a = nullReceiver ? null : New(method.DeclaringType!), b = nullReceiver ? null : New(recovered.GetType(method.DeclaringType!.FullName!)!);
                if (!nullReceiver) { Set(a, "Effects", 7); Set(b, "Effects", 7); }
                int allocations = 0;
                recovered.GetType("Recovered")!.GetField("ExceptionAllocationObserver")!.SetValue(null,
                    (Action<int, string, string>)((token, p, m) => {
                        if (b == null || (int)Get(b, "Effects") != 1 || !ReferenceEquals(p, parameter) || !ReferenceEquals(m, message))
                            throw new Exception("Exception constructor arguments or allocation position changed");
                        allocations++;
                    }));
                Check(recovered, method, a, b, new object[] { parameter, message }, new object[] { parameter, message }, () => {
                    if (!nullReceiver)
                    {
                        SameFields(a, b, "Effects");
                        if ((int)Get(a, "Effects") != 2) throw new Exception("Exception allocation lost following effect");
                    }
                    if (allocations != (nullReceiver ? 0 : 1)) throw new Exception("Exception allocation missing or duplicated");
                }, nullReceiver ? null : EqualObject);
            }
            recovered.GetType("Recovered")!.GetField("ExceptionAllocationObserver")!.SetValue(null, null);
        }
        foreach (string name in new[] { "Literal", "Null" })
            Check(recovered, methods.Values.Single(m => m.Name == name), null, null, Array.Empty<object>(), Array.Empty<object>(), equal: EqualObject);
        MethodInfo body = recovered.GetType("Recovered")!.GetMethod($"M_{make.MetadataToken:X8}")!;
        object first = body.Invoke(null, new object[] { "parameter", "message" });
        object second = body.Invoke(null, new object[] { "parameter", "message" });
        if (ReferenceEquals(first, second)) throw new Exception("Exception allocation reused object identity");
        foreach (string parameter in values)
        foreach (string message in values)
        foreach (bool nullReceiver in new[] { false, true })
        {
            MethodInfo method = methods.Values.Single(m => m.Name == "Raise");
            object a = nullReceiver ? null : New(method.DeclaringType!), b = nullReceiver ? null : New(recovered.GetType(method.DeclaringType!.FullName!)!);
            Check(recovered, method, a, b, new object[] { parameter, message }, new object[] { parameter, message },
                () => { if (!nullReceiver) { SameFields(a, b, "Effects"); if ((int)Get(a, "Effects") != 1) throw new Exception("Throw lost prior effect"); } },
                detailedExceptions: true);
        }
        MethodInfo existing = methods.Values.Single(m => m.Name == "ThrowExisting");
        foreach (bool nullError in new[] { false, true })
        foreach (bool nullReceiver in new[] { false, true })
        {
            object a = nullReceiver ? null : New(existing.DeclaringType!), b = nullReceiver ? null : New(recovered.GetType(existing.DeclaringType!.FullName!)!);
            var error = nullError ? null : new ArgumentOutOfRangeException("original", "message");
            Check(recovered, existing, a, b, new object[] { error }, new object[] { error },
                () => { if (!nullReceiver) SameFields(a, b, "Effects"); }, detailedExceptions: true);
            if (!nullError && !nullReceiver)
            {
                Exception Catch(Func<object> action)
                {
                    try { action(); }
                    catch (Exception e) { return e is TargetInvocationException t ? t.InnerException! : e; }
                    throw new Exception("Expected the original exception reference");
                }
                var originalError = Catch(() => Original(existing, a, new object[] { error }));
                var loweredError = Catch(() => recovered.GetType("Recovered")!.GetMethod($"M_{existing.MetadataToken:X8}")!.Invoke(null, new[] { b, error }));
                if (!ReferenceEquals(originalError, error) || !ReferenceEquals(loweredError, error))
                    throw new Exception("Throw substituted a different exception object");
            }
        }
        Check(recovered, methods.Values.Single(m => m.Name == "ThrowNull"), null, null,
            Array.Empty<object>(), Array.Empty<object>(), detailedExceptions: true);
        Console.WriteLine("Exceptions: allocation/arguments/identity/order and throw/null/prior state; detailed exception parameters preserved; EH not included.");
    }

    static void Strings(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        string[] names = { "Empty", "Ascii", "Escapes", "Units", "Shared", "SharedAgain", "Identity", "Choose", "Null", "Replace", "SetLiteral" };
        if (methods.Count != names.Length || !methods.Values.Select(m => m.Name).Order().SequenceEqual(names.Order()))
            throw new Exception("String selection drift");
        bool SameUnits(object x, object y) => x == null ? y == null : x is string left && y is string right
            && left.Select(c => (ushort)c).SequenceEqual(right.Select(c => (ushort)c));
        foreach (string name in new[] { "Empty", "Ascii", "Escapes", "Units", "Shared", "SharedAgain", "Null" })
            Check(recovered, methods.Values.Single(m => m.Name == name), null, null, Array.Empty<object>(), Array.Empty<object>(), equal: SameUnits);
        string[] values = { null, "", new string(new[] { 's', 'a', 'm', 'e' }), "\0", "\ud800X\udfff", "\ud83d\ude00" };
        foreach (string value in values)
            Check(recovered, methods.Values.Single(m => m.Name == "Identity"), null, null,
                new object[] { value }, new object[] { value }, equal: (x, y) => ReferenceEquals(x, value) && ReferenceEquals(y, value));
        foreach (bool first in new[] { false, true })
        foreach (string left in values)
        foreach (string right in values)
            Check(recovered, methods.Values.Single(m => m.Name == "Choose"), null, null,
                new object[] { first, left, right }, new object[] { first, left, right },
                equal: (x, y) => ReferenceEquals(x, first ? left : right) && ReferenceEquals(y, first ? left : right));
        MethodInfo replace = methods.Values.Single(m => m.Name == "Replace");
        Type cell = replace.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        foreach (string initial in values)
        foreach (string replacement in values)
        {
            object a = New(cell), b = New(clone);
            Set(a, "Text", initial); Set(b, "Text", initial);
            Check(recovered, replace, a, b, new object[] { replacement }, new object[] { replacement }, () => {
                if (!ReferenceEquals(Get(a, "Text"), replacement) || !ReferenceEquals(Get(b, "Text"), replacement))
                    throw new Exception("String field replacement lost identity");
            }, (x, y) => ReferenceEquals(x, initial) && ReferenceEquals(y, initial));
        }
        int loads = 0;
        foreach (bool nullReceiver in new[] { false, true })
        {
            object a = nullReceiver ? null : New(cell), b = nullReceiver ? null : New(clone);
            int before = loads;
            recovered.GetType("Recovered")!.GetField("StringLoadObserver")!.SetValue(null, (Action<int, string>)((token, units) => {
                if (b == null || (int)Get(b, "Effects") != 1) throw new Exception("Literal load moved before its field effect");
                string original = cell.Module.ResolveString(token);
                if (!original.Select(c => (ushort)c).SequenceEqual(units.Select(c => (ushort)c)))
                    throw new Exception("Output literal UTF-16 units differ from metadata");
                loads++;
            }));
            Check(recovered, methods.Values.Single(m => m.Name == "SetLiteral"), a, b,
                Array.Empty<object>(), Array.Empty<object>(), () => {
                    if (!nullReceiver) { SameFields(a, b, "Effects"); if (!SameUnits(Get(a, "Text"), Get(b, "Text"))) throw new Exception("String field units mismatch"); }
                }, nullReceiver ? null : SameUnits);
            if (loads != before + (nullReceiver ? 0 : 1)) throw new Exception("String load lost exception position");
        }
        Check(recovered, replace, null, null, new object[] { null }, new object[] { null });
        recovered.GetType("Recovered")!.GetField("StringLoadObserver")!.SetValue(null, null);
        foreach (string name in new[] { "Shared", "SharedAgain", "Units" })
        {
            MethodInfo method = methods.Values.Single(m => m.Name == name);
            MethodInfo body = recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!;
            object first = body.Invoke(null, Array.Empty<object>()), second = body.Invoke(null, Array.Empty<object>());
            if (!ReferenceEquals(first, second)) throw new Exception("Literal adapter allocated on each load");
        }
        Console.WriteLine("Strings: exact UTF-16 units, embedded null/unpaired surrogates, reference state and owned literal runtime policy checked.");
    }

    static void Clock(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        if (methods.Count != 1 || methods.Values.Single().Name != "ClockValue")
            throw new Exception("Clock fixture selection drift");
        MethodInfo method = methods.Values.Single();
        MethodInfo body = recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!;
        int calls = 0;
        foreach (int value in new[] { int.MinValue, -1, 0, 1, int.MaxValue })
        {
            recovered.GetType("Recovered")!.GetField("TickCount")!.SetValue(null,
                (Func<int>)(() => { calls++; return value; }));
            object actual = body.Invoke(null, Array.Empty<object>());
            if (!Equals(actual, value)) throw new Exception("Clock bits changed");
            checks++;
        }
        if (calls != 5) throw new Exception("Clock read missing or duplicated");
        // The external operation is the runtime's exact getter, not guessed host time.
        MethodInfo target = typeof(Environment).GetProperty("TickCount")!.GetMethod!;
        if (Original(target, null, Array.Empty<object>()) is not int)
            throw new Exception("Runtime clock contract mismatch");
        Console.WriteLine("Clock: exact target validated; controlled signed boundary values read once each; distinct real times not compared.");
    }

    static void Constructors(Assembly recovered, Dictionary<int, MethodBase> methods)
    {
        if (methods.Count != 2 || methods.Values.Any(m => m is not ConstructorInfo || m.GetParameters().Length != 1)
            || !methods.Values.Select(m => m.DeclaringType!.FullName).Order().SequenceEqual(
                new[] { "ConstructorCases.Base", "ConstructorCases.Root" }.Order()))
            throw new Exception("Core constructor fixture selection drift");
        Action<object> exactCore = CoreObjectCall();
        exactCore(null); // Direct core call does not acquire callvirt's receiver null check on this runtime.
        exactCore(new object());
        foreach (MethodBase method in methods.Values)
        foreach (int value in new[] { int.MinValue, -1, 0, 1, int.MaxValue })
        foreach (bool dirty in new[] { false, true })
        {
            Type cell = method.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
            bool root = cell.Name == "Root";
            object a = New(cell), b = New(clone);
            uint[] oldLeft = new[] { 17u }, oldRight = new[] { 17u };
            if (dirty)
            {
                if (root)
                {
                    Set(a, "Before", -31); Set(b, "Before", -31);
                    Set(a, "Values", oldLeft); Set(b, "Values", oldRight);
                    Set(a, "After", -12); Set(b, "After", -12);
                    Set(a, "Effects", -5); Set(b, "Effects", -5);
                }
                else { Set(a, "BaseValue", -77); Set(b, "BaseValue", -77); }
            }
            int calls = 0;
            Action<object> callback = self => {
                if (!ReferenceEquals(self, b)) throw new Exception("Core constructor changed the receiver identity");
                if (root)
                {
                    uint[] state = (uint[])Get(self, "Values");
                    if ((int)Get(self, "Before") != 9 || state == null || state.Length != 3 || state.Any(n => n != 0)
                        || ReferenceEquals(state, oldRight) || (int)Get(self, "After") != (dirty ? -12 : 0)
                        || (int)Get(self, "Effects") != (dirty ? -5 : 0))
                        throw new Exception("Core constructor moved across field initialization");
                }
                else if ((int)Get(self, "BaseValue") != (dirty ? -77 : 0))
                    throw new Exception("Readonly write moved before core constructor call");
                calls++;
                exactCore(self);
            };
            recovered.GetType("Recovered")!.GetField("CoreConstructorCall")!.SetValue(null, callback);
            Check(recovered, method, a, b, new object[] { value }, new object[] { value }, () => {
                if (calls != 1) throw new Exception("Core constructor call missing or duplicated");
                SameFields(a, b, root ? new[] { "Before", "After", "Effects" } : new[] { "BaseValue" });
                if (root)
                {
                    uint[] left = (uint[])Get(a, "Values"), right = (uint[])Get(b, "Values");
                    SameArray(left, right);
                    if (left.Length != 3 || ReferenceEquals(left, oldLeft) || ReferenceEquals(right, oldRight))
                        throw new Exception("Constructor allocation identity mismatch");
                }
            }, (x, y) => x == null && y == null);
        }
        foreach (MethodBase method in methods.Values)
        {
            int calls = 0;
            recovered.GetType("Recovered")!.GetField("CoreConstructorCall")!.SetValue(null,
                (Action<object>)(self => { calls++; exactCore(self); }));
            Check(recovered, method, null, null, new object[] { 7 }, new object[] { 7 });
            if (calls != (method.DeclaringType!.Name == "Base" ? 1 : 0))
                throw new Exception("Constructor null failure or core call timing changed");
        }
        Console.WriteLine("Core Object constructor: exact runtime call, no allocation or added null check; pre/post field ordering observed.");
    }

    static Action<object, Func<int>> ControlledOriginalDefault(ConstructorInfo constructor, ConstructorInfo delegated)
    {
        // Decode the original bounded body, substituting only the external clock call.
        // The oracle is independent of the decompiler's output and does not rewrite the input image.
        MethodBody body = constructor.GetMethodBody()!;
        if (body.LocalVariables.Count != 0 || body.ExceptionHandlingClauses.Count != 0)
            throw new Exception("Controlled default oracle requires a straight-line body without locals/EH");
        byte[] code = body.GetILAsByteArray()!;
        var dm = new DynamicMethod("ControlledOriginalDefault", typeof(void), new[] { typeof(object), typeof(Func<int>) }, constructor.Module, true);
        ILGenerator il = dm.GetILGenerator();
        int stage = 0;
        for (int offset = 0; offset < code.Length;)
        {
            byte op = code[offset++];
            if (op == 0x00) { il.Emit(OpCodes.Nop); continue; }
            if (op == 0x02 && stage == 0)
            {
                il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Castclass, constructor.DeclaringType!); stage = 1;
            }
            else if (op == 0x28 && (stage == 1 || stage == 2) && offset + 4 <= code.Length)
            {
                int token = BitConverter.ToInt32(code, offset); offset += 4;
                MethodBase target = constructor.Module.ResolveMethod(token)!;
                if (stage == 1)
                {
                    if (target is not MethodInfo clock || clock.DeclaringType != typeof(Environment)
                        || clock.Name != "get_TickCount" || !clock.IsStatic || clock.ReturnType != typeof(int)
                        || clock.GetParameters().Length != 0)
                        throw new Exception("Original default clock target drift");
                    il.Emit(OpCodes.Ldarg_1);
                    il.Emit(OpCodes.Callvirt, typeof(Func<int>).GetMethod("Invoke")!);
                    stage = 2;
                }
                else
                {
                    if (target != delegated) throw new Exception("Original delegated constructor target drift");
                    il.Emit(OpCodes.Call, delegated); stage = 3;
                }
            }
            else if (op == 0x2a && stage == 3 && offset == code.Length)
            {
                il.Emit(OpCodes.Ret); stage = 4;
            }
            else throw new Exception($"Unadmitted original default instruction at {offset - 1}");
        }
        if (stage != 4) throw new Exception("Incomplete original default oracle");
        return (Action<object, Func<int>>)dm.CreateDelegate(typeof(Action<object, Func<int>>));
    }

    static void DefaultConstructor(Assembly recovered, Dictionary<int, MethodBase> methods)
    {
        ConstructorInfo entry = (ConstructorInfo)methods.Values.Single(m => m is ConstructorInfo && m.GetParameters().Length == 0);
        ConstructorInfo delegated = (ConstructorInfo)methods.Values.Single(m => m is ConstructorInfo && m.GetParameters().Length == 1);
        Type cell = entry.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        bool random = cell.FullName == "OpenRA.Support.MersenneTwister";
        if (methods.Count != (random ? 4 : 2) || methods.Values.Any(m => m.DeclaringType != cell)
            || delegated.GetParameters()[0].ParameterType != typeof(int))
            throw new Exception("Default constructor selection drift");
        Action<object, Func<int>> original = ControlledOriginalDefault(entry, delegated);
        Action<object> exactCore = CoreObjectCall();
        Type loweredType = recovered.GetType("Recovered")!;
        MethodInfo loweredEntry = loweredType.GetMethod($"M_{entry.MetadataToken:X8}")!;
        string arrayField = random ? "mt" : "Values";
        string[] scalars = random ? new[] { "index", "Last", "TotalCount" } : new[] { "Before", "After", "Effects" };
        foreach (int clockValue in new[] { int.MinValue, -1, 0, 1, int.MaxValue })
        foreach (bool dirty in new[] { false, true })
        foreach (bool nullReceiver in new[] { false, true })
        {
            object a = nullReceiver ? null : New(cell), b = nullReceiver ? null : New(clone);
            uint[] oldLeft = dirty && !nullReceiver ? new[] { 17u } : null;
            uint[] oldRight = dirty && !nullReceiver ? new[] { 17u } : null;
            if (dirty && !nullReceiver)
            {
                Set(a, arrayField, oldLeft); Set(b, arrayField, oldRight);
                foreach (string name in scalars) { Set(a, name, -23); Set(b, name, -23); }
            }
            void AssertBefore(object self, uint[] old)
            {
                if (self == null) return;
                if (!ReferenceEquals(Get(self, arrayField), old)
                    || scalars.Any(name => (int)Get(self, name) != (dirty ? -23 : 0)))
                    throw new Exception("Initialization moved before clock/delegated call");
            }
            int originalReads = 0, reads = 0, delegations = 0, cores = 0;
            Func<int> originalClock = () => { AssertBefore(a, oldLeft); originalReads++; return clockValue; };
            loweredType.GetField("TickCount")!.SetValue(null, (Func<int>)(() => {
                AssertBefore(b, oldRight);
                if (delegations != 0 || cores != 0) throw new Exception("Clock read reordered");
                reads++; return clockValue;
            }));
            loweredType.GetField("ConstructorCallObserver")!.SetValue(null, (Action<int, object, object[]>)((token, self, values) => {
                if (token != delegated.MetadataToken || !ReferenceEquals(self, b) || reads != 1 || cores != 0
                    || values.Length != 1 || !Equals(values[0], clockValue))
                    throw new Exception("Delegated target/receiver/clock bits changed");
                AssertBefore(self, oldRight); delegations++;
            }));
            loweredType.GetField("CoreConstructorCall")!.SetValue(null, (Action<object>)(self => {
                if (!ReferenceEquals(self, b) || reads != 1 || delegations != 1)
                    throw new Exception("Core call occurred before delegation");
                uint[] state = (uint[])Get(self, arrayField);
                if (state == null || state.Length != (random ? 624 : 3) || state.Any(n => n != 0)
                    || ReferenceEquals(state, oldRight)) throw new Exception("Default constructor allocation timing changed");
                cores++; exactCore(self);
            }));
            object expected = Outcome(() => { original(a, originalClock); return null; });
            object actual = Outcome(() => loweredEntry.Invoke(null, new[] { b }));
            if (!Equals(expected, actual) || (!nullReceiver && expected != null))
                throw new Exception($"Default constructor outcome mismatch: {expected} / {actual}");
            if (originalReads != 1 || reads != 1 || delegations != 1 || cores != (nullReceiver ? 0 : 1))
                throw new Exception("Default constructor clock/delegation/core count mismatch");
            if (!nullReceiver)
            {
                SameFields(a, b, scalars);
                uint[] left = (uint[])Get(a, arrayField), right = (uint[])Get(b, arrayField);
                SameArray(left, right);
                if (left.Length != (random ? 624 : 3) || ReferenceEquals(left, oldLeft) || ReferenceEquals(right, oldRight))
                    throw new Exception("Default construction changed array identity/extent");
                if (random && !dirty)
                {
                    MethodInfo next = (MethodInfo)methods.Values.Single(m => m.Name == "Next");
                    for (int draw = 0; draw < 3; draw++)
                        Check(recovered, next, a, b, Array.Empty<object>(), Array.Empty<object>(), () => {
                            SameFields(a, b, scalars); SameArray((uint[])Get(a, arrayField), (uint[])Get(b, arrayField));
                        }, (x, y) => x is int && y is int && Equals(x, y));
                }
            }
            checks++;
        }
        loweredType.GetField("ConstructorCallObserver")!.SetValue(null, null);
        Console.WriteLine("Default: original IL replay changes only clock boundary; recovered bodies keep one clock/delegation/core call in order, including null receiver paths.");
    }

    static void RandomSeed(Assembly recovered, Dictionary<int, MethodBase> methods)
    {
        if (methods.Count != 3 || methods.Values.Count(m => m is ConstructorInfo) != 1)
            throw new Exception("Random seed selection drift");
        ConstructorInfo constructor = (ConstructorInfo)methods.Values.Single(m => m is ConstructorInfo);
        MethodInfo next = (MethodInfo)methods.Values.Single(m => m.Name == "Next");
        MethodInfo generate = (MethodInfo)methods.Values.Single(m => m.Name == "Generate");
        Type cell = constructor.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        if (constructor.GetParameters().Length != 1 || constructor.GetParameters()[0].ParameterType != typeof(int)
            || methods.Values.Any(m => m.DeclaringType != cell)) throw new Exception("Random seed signature drift");
        Action<object> exactCore = CoreObjectCall();
        string[] scalars = { "index", "Last", "TotalCount" };
        object[] noArgs = Array.Empty<object>();
        void SameState(object a, object b)
        {
            SameFields(a, b, scalars);
            uint[] left = (uint[])Get(a, "mt"), right = (uint[])Get(b, "mt");
            if (left == null || right == null || left.Length != 624 || right.Length != 624)
                throw new Exception("Seed initialization state extent mismatch");
            for (int i = 0; i < 624; i++)
                if (left[i] != right[i]) throw new Exception($"Seed state mismatch at word {i}");
        }
        foreach (int seed in new[] { 0, 1, -1, int.MinValue, int.MaxValue })
        foreach (bool dirty in new[] { false, true })
        {
            object a = New(cell), b = New(clone);
            uint[] oldLeft = new[] { 17u }, oldRight = new[] { 17u };
            if (dirty)
            {
                Set(a, "mt", oldLeft); Set(b, "mt", oldRight);
                foreach (string name in scalars) { Set(a, name, -23); Set(b, name, -23); }
            }
            int coreCalls = 0;
            recovered.GetType("Recovered")!.GetField("CoreConstructorCall")!.SetValue(null, (Action<object>)(self => {
                uint[] state = (uint[])Get(self, "mt");
                if (!ReferenceEquals(self, b) || state == null || state.Length != 624
                    || state.Any(n => n != 0) || ReferenceEquals(state, oldRight)
                    || scalars.Any(name => (int)Get(self, name) != (dirty ? -23 : 0)))
                    throw new Exception("Seed initialization moved across its core base call");
                coreCalls++;
                exactCore(self);
            }));
            Check(recovered, constructor, a, b, new object[] { seed }, new object[] { seed }, () => {
                if (coreCalls != 1 || ReferenceEquals(Get(a, "mt"), oldLeft) || ReferenceEquals(Get(b, "mt"), oldRight))
                    throw new Exception("Seed allocation or core call identity mismatch");
                SameState(a, b);
            }, (x, y) => x == null && y == null);
            if (!dirty)
            {
                object freshOracle = constructor.Invoke(new object[] { seed });
                SameState(freshOracle, b); // Compare normal object construction without copying any original state.
                for (int draw = 0; draw < 1250; draw++)
                {
                    uint[] left = (uint[])Get(a, "mt"), right = (uint[])Get(b, "mt");
                    Check(recovered, next, a, b, noArgs, noArgs, () => {
                        SameState(a, b);
                        if (!ReferenceEquals(Get(a, "mt"), left) || !ReferenceEquals(Get(b, "mt"), right))
                            throw new Exception("Next replaced the seed state array");
                    }, (x, y) => x is int && y is int && Equals(x, y));
                }
                Check(recovered, generate, a, b, noArgs, noArgs, () => SameState(a, b), (x, y) => x == null && y == null);
            }
        }
        int nullCoreCalls = 0;
        recovered.GetType("Recovered")!.GetField("CoreConstructorCall")!.SetValue(null,
            (Action<object>)(self => { nullCoreCalls++; exactCore(self); }));
        Check(recovered, constructor, null, null, new object[] { 1 }, new object[] { 1 });
        if (nullCoreCalls != 0) throw new Exception("Seed constructor null failure moved past its field store");
        Console.WriteLine("Random seed: all 624 words and scalars; recovered allocation/init body; 1,250 draws per boundary seed; no original-state copying.");
    }

    static void RandomRanges(Assembly recovered, Dictionary<int, MethodBase> methods)
    {
        if (methods.Count != 5 || methods.Values.Count(m => m is ConstructorInfo) != 1)
            throw new Exception("Random range selection drift");
        ConstructorInfo constructor = (ConstructorInfo)methods.Values.Single(m => m is ConstructorInfo);
        MethodInfo range = (MethodInfo)methods.Values.Single(m => m.Name == "Next" && m.GetParameters().Length == 2);
        MethodInfo highOnly = (MethodInfo)methods.Values.Single(m => m.Name == "Next" && m.GetParameters().Length == 1);
        Type cell = constructor.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        if (methods.Values.Any(m => m.DeclaringType != cell)) throw new Exception("Random range owner drift");
        Type bodies = recovered.GetType("Recovered")!;
        bodies.GetField("CoreConstructorCall")!.SetValue(null, CoreObjectCall());
        MethodInfo initialize = bodies.GetMethod($"M_{constructor.MetadataToken:X8}")!;
        int[] bounds = { int.MinValue, int.MinValue + 1, -17, -1, 0, 1, 2, 17, int.MaxValue - 1, int.MaxValue };
        string[] scalars = { "index", "Last", "TotalCount" };
        var cases = bounds.SelectMany(low => bounds.Select(high => (Method: range, Low: low, High: high,
            Args: new object[] { low, high })))
            .Concat(bounds.Select(high => (Method: highOnly, Low: 0, High: high, Args: new object[] { high }))).ToArray();
        void SameState(object a, object b, uint[] left, uint[] right)
        {
            SameFields(a, b, scalars);
            SameArray(left, right);
            if (!ReferenceEquals(Get(a, "mt"), left) || !ReferenceEquals(Get(b, "mt"), right)
                || left.Length != 624 || right.Length != 624)
                throw new Exception("Range call changed array identity/extent");
        }
        foreach (int seed in new[] { 0, 1, -1, int.MinValue, int.MaxValue })
        foreach (int index in new[] { 0, 1, 623 })
        foreach (int total in new[] { 0, int.MaxValue })
        foreach (var test in cases)
        {
            object a = New(cell), b = New(clone);
            if (Original(constructor, a, new object[] { seed }) != null
                || initialize.Invoke(null, new object[] { b, seed }) != null)
                throw new Exception("Range setup initialization failed");
            Set(a, "index", index); Set(b, "index", index);
            Set(a, "Last", -19); Set(b, "Last", -19);
            Set(a, "TotalCount", total); Set(b, "TotalCount", total);
            uint[] left = (uint[])Get(a, "mt"), right = (uint[])Get(b, "mt");
            SameState(a, b, left, right);
            uint[] before = (uint[])left.Clone();
            // Retain the original unchecked subtraction, even when ordered bounds wrap.
            bool noDraw = test.High < test.Low || unchecked(test.High - test.Low) <= 1;
            Check(recovered, test.Method, a, b, test.Args, test.Args, () => {
                SameState(a, b, left, right);
                if (noDraw && ((int)Get(a, "index") != index || (int)Get(a, "Last") != -19
                    || (int)Get(a, "TotalCount") != total || !before.SequenceEqual(left)))
                    throw new Exception("Invalid or degenerate range consumed random state");
            }, (x, y) => {
                if (test.High < test.Low)
                    return x is Fault fault && fault.Type == typeof(ArgumentOutOfRangeException).FullName
                        && fault.Parameter == "high" && fault.ActualValue == null && Equals(x, y);
                return x is int && y is int && Equals(x, y);
            }, detailedExceptions: true);
        }
        foreach (var test in cases)
            Check(recovered, test.Method, null, null, test.Args, test.Args, detailedExceptions: true);
        Console.WriteLine("Ranges: negative/degenerate/ordered-overflow bounds, detailed high parameter faults, full state and generation boundary effects; recovered seed/state dependencies only.");
    }

    static void RandomFloat(Assembly recovered, Dictionary<int, MethodBase> methods)
    {
        if (methods.Count != 4 || methods.Values.Count(m => m is ConstructorInfo) != 1)
            throw new Exception("Random float selection drift");
        ConstructorInfo constructor = (ConstructorInfo)methods.Values.Single(m => m is ConstructorInfo);
        MethodInfo method = (MethodInfo)methods.Values.Single(m => m.Name == "NextFloat");
        Type cell = constructor.DeclaringType!, clone = recovered.GetType(cell.FullName!)!;
        if (methods.Values.Any(m => m.DeclaringType != cell) || method.ReturnType != typeof(float)
            || method.GetParameters().Length != 0) throw new Exception("Random float signature drift");
        Type bodies = recovered.GetType("Recovered")!;
        bodies.GetField("CoreConstructorCall")!.SetValue(null, CoreObjectCall());
        MethodInfo initialize = bodies.GetMethod($"M_{constructor.MetadataToken:X8}")!;
        string[] scalars = { "index", "Last", "TotalCount" };
        object[] noArgs = Array.Empty<object>();
        foreach (int seed in new[] { 0, 1, -1, int.MinValue, int.MaxValue })
        {
            object a = New(cell), b = New(clone);
            if (Original(constructor, a, new object[] { seed }) != null
                || initialize.Invoke(null, new object[] { b, seed }) != null)
                throw new Exception("Float gate seed initialization failed");
            uint[] left = (uint[])Get(a, "mt"), right = (uint[])Get(b, "mt");
            SameArray(left, right);
            for (int draw = 0; draw < 1250; draw++)
            {
                int priorCount = (int)Get(b, "TotalCount");
                int absoluteCalls = 0;
                bodies.GetField("AbsoluteCallObserver")!.SetValue(null, (Action<int, float>)((token, value) => {
                    if ((int)Get(b, "TotalCount") != unchecked(priorCount + 1)
                        || float.IsNaN(value) || value < 0f || value > 1f)
                        throw new Exception("Absolute call moved before random state update or changed its input");
                    absoluteCalls++;
                }));
                Check(recovered, method, a, b, noArgs, noArgs, () => {
                    SameFields(a, b, scalars); SameArray(left, right);
                    if (left.Length != 624 || right.Length != 624 || !ReferenceEquals(Get(a, "mt"), left)
                        || !ReferenceEquals(Get(b, "mt"), right) || absoluteCalls != 1)
                        throw new Exception("NextFloat state identity or absolute-call count mismatch");
                }, (x, y) => x is float && y is float && SameFloatBits(x, y));
            }
        }
        bodies.GetField("AbsoluteCallObserver")!.SetValue(null, null);
        foreach (int length in new[] { -1, 0, 1, 624 })
        foreach (int index in new[] { -1, 0, 1, 623, 624 })
        foreach (int total in new[] { int.MinValue, int.MaxValue })
        {
            object a = New(cell), b = New(clone);
            uint[] left = length < 0 ? null : new uint[length], right = (uint[])left?.Clone();
            Set(a, "mt", left); Set(b, "mt", right);
            Set(a, "index", index); Set(b, "index", index);
            Set(a, "Last", -19); Set(b, "Last", -19);
            Set(a, "TotalCount", total); Set(b, "TotalCount", total);
            Check(recovered, method, a, b, noArgs, noArgs, () => {
                SameFields(a, b, scalars); SameArray(left, right);
            }, SameFloatBits, detailedExceptions: true);
        }
        Check(recovered, method, null, null, noArgs, noArgs, detailedExceptions: true);
        Console.WriteLine("NextFloat: 1,250 bitwise results per seed, all state after each call, exact Abs(float) dependency; local .NET 10 F carrier only.");
    }

    static void RandomState(Assembly recovered, Dictionary<int, MethodInfo> methods)
    {
        if (methods.Count != 2 || !methods.Values.Select(m => m.Name).Order().SequenceEqual(new[] { "Generate", "Next" }.Order())
            || methods.Values.Any(m => m.IsStatic || m.GetParameters().Length != 0))
            throw new Exception("Random state method selection drift");
        MethodInfo generate = methods.Values.Single(m => m.Name == "Generate");
        MethodInfo next = methods.Values.Single(m => m.Name == "Next");
        Type cell = next.DeclaringType!;
        if (generate.DeclaringType != cell) throw new Exception("Random methods have different owners");
        Type clone = recovered.GetType(cell.FullName!)!;
        ConstructorInfo seedConstructor = cell.GetConstructor(new[] { typeof(int) })
            ?? throw new Exception("Missing independent seed oracle");
        string[] scalars = { "index", "Last", "TotalCount" };
        object[] noArgs = Array.Empty<object>();
        object CopyState(object original)
        {
            object copy = New(clone);
            Set(copy, "mt", ((uint[])Get(original, "mt"))?.Clone());
            foreach (string name in scalars) Set(copy, name, Get(original, name));
            return copy;
        }
        void Step(MethodInfo method, object a, object b, bool returnsInt = false, bool expectSuccess = false)
        {
            uint[] left = (uint[])Get(a, "mt"), right = (uint[])Get(b, "mt");
            Check(recovered, method, a, b, noArgs, noArgs, () => {
                SameFields(a, b, scalars);
                if (!ReferenceEquals(Get(a, "mt"), left) || !ReferenceEquals(Get(b, "mt"), right))
                    throw new Exception("Random method replaced the state array");
                if (left == null ? right != null : right == null || left.Length != right.Length)
                    throw new Exception("Random state extent mismatch");
                if (left != null)
                    for (int i = 0; i < left.Length; i++)
                        if (left[i] != right[i]) throw new Exception($"Random state mismatch at word {i}");
            }, returnsInt ? (x, y) => x is int && y is int && Equals(x, y)
                : expectSuccess ? (x, y) => x == null && y == null : null);
        }
        foreach (int seed in new[] { 0, 1, -1, int.MinValue, int.MaxValue })
        {
            // Setup only: copy the independent original constructor's initial state.
            // This is not a recovered seed constructor or an initialization gate.
            object a = seedConstructor.Invoke(new object[] { seed }), b = CopyState(a);
            if (((uint[])Get(a, "mt")).Length != 624) throw new Exception("Pinned state extent drift");
            for (int pass = 0; pass < 3; pass++) Step(generate, a, b, expectSuccess: true);
            a = seedConstructor.Invoke(new object[] { seed }); b = CopyState(a);
            for (int draw = 0; draw < 1250; draw++) Step(next, a, b, returnsInt: true);
        }
        foreach (int pattern in new[] { 0, 1, 2, 3 })
        foreach (int length in new[] { -1, 0, 1, 623, 624, 625 })
        {
            uint[] state = length < 0 ? null : Enumerable.Range(0, length)
                .Select(i => pattern == 0 ? 0u : pattern == 1 ? uint.MaxValue : pattern == 2
                    ? (i % 2 == 0 ? 0x80000000u : 0x7fffffffu) : unchecked((uint)i * 2654435761u)).ToArray();
            foreach (MethodInfo method in new[] { generate, next })
            foreach (int index in new[] { int.MinValue, -1, 0, 1, 623, 624, int.MaxValue })
            foreach (int total in new[] { int.MinValue, int.MaxValue })
            {
                object a = New(cell), b = New(clone);
                Set(a, "mt", state?.Clone()); Set(b, "mt", state?.Clone());
                Set(a, "index", index); Set(b, "index", index);
                Set(a, "Last", -19); Set(b, "Last", -19);
                Set(a, "TotalCount", total); Set(b, "TotalCount", total);
                Step(method, a, b);
            }
        }
        Check(recovered, generate, null, null, noArgs, noArgs);
        Check(recovered, next, null, null, noArgs, noArgs);
        Console.WriteLine("Random state: full words/scalars after each step; 1,250 consecutive draws per seed; original constructor used only as setup oracle, not recovered.");
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
        foreach (int length in new[] { int.MinValue, -1, 0, 1, 4, 624 })
        {
            bool EqualAllocation(object x, object y)
            {
                if (length < 0) return Equals(x, y) && x is string;
                if (x is not Array left || y is not Array right || ReferenceEquals(left, right)
                    || left.Length != length || right.Length != length) return false;
                SameArray(left, right);
                return left.Cast<object>().All(v => Equals(v, left.GetType() == typeof(uint[]) ? (object)0u : 0));
            }
            foreach (string name in new[] { "AllocateSigned", "AllocateUnsigned" })
            {
                Check(recovered, methods.Values.Single(m => m.Name == name), null, null,
                    new object[] { length }, new object[] { length }, equal: EqualAllocation);
                if (length >= 0)
                {
                    MethodInfo method = methods.Values.Single(m => m.Name == name);
                    object left1 = Original(method, null, new object[] { length });
                    object left2 = Original(method, null, new object[] { length });
                    MethodInfo lowered = recovered.GetType("Recovered")!.GetMethod($"M_{method.MetadataToken:X8}")!;
                    object right1 = lowered.Invoke(null, new object[] { length });
                    object right2 = lowered.Invoke(null, new object[] { length });
                    if (ReferenceEquals(left1, left2) || ReferenceEquals(right1, right2))
                        throw new Exception("Allocation reused an array identity");
                }
            }
            foreach (uint[] state in states)
            {
                object a = New(cell), b = New(clone);
                uint[] left = (uint[])state?.Clone(), right = (uint[])state?.Clone();
                Set(a, "Values", left); Set(b, "Values", right);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                Check(recovered, methods.Values.Single(m => m.Name == "AllocateAfterEffect"), a, b,
                    new object[] { length }, new object[] { length }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right);
                        uint[] al = (uint[])Get(a, "Values"), ar = (uint[])Get(b, "Values");
                        if (length < 0)
                        {
                            if ((int)Get(a, "Effects") != 8 || !ReferenceEquals(al, left) || !ReferenceEquals(ar, right))
                                throw new Exception("Failed allocation moved effects or replaced the old array");
                        }
                        else if ((int)Get(a, "Effects") != 9 || al == null || ar == null
                            || ReferenceEquals(al, left) || ReferenceEquals(ar, right) || al.Length != length || ar.Length != length
                            || al.Any(v => v != 0) || ar.Any(v => v != 0))
                            throw new Exception("Array allocation lost identity or zero initialization");
                    }, (x, y) => length < 0 ? Equals(x, y) && x is string : ReferenceEquals(x, left) && ReferenceEquals(y, right));
            }
            object ca = New(cell), cb = New(clone);
            Set(ca, "Effects", 7); Set(cb, "Effects", 7);
            Check(recovered, methods.Values.Single(m => m.Name == "AllocateThenEffect"), ca, cb,
                new object[] { length }, new object[] { length }, () => {
                    SameFields(ca, cb, "Effects");
                    if ((int)Get(ca, "Effects") != (length < 0 ? 7 : 8))
                        throw new Exception("Call moved across allocation failure");
                }, EqualAllocation);
        }
        foreach (string name in new[] { "AllocateAfterEffect", "AllocateThenEffect" })
        foreach (int length in new[] { -1, 0, 1 })
            Check(recovered, methods.Values.Single(m => m.Name == name), null, null,
                new object[] { length }, new object[] { length });
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
            "LengthSigned", "LengthUnsigned", "WideLength", "LengthAfterEffect", "LengthSnapshot", "CountLoop",
            "AllocateSigned", "AllocateUnsigned", "AllocateAfterEffect", "AllocateThenEffect"
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
        foreach (int length in new[] { -1, 0, 1, 3 })
        foreach (int index in indices)
        foreach (uint value in new[] { 0u, 1u, 0x80000000u, uint.MaxValue })
        {
            foreach (string name in new[] { "WriteUnsigned", "WriteSignedBits", "UpdateViaDup" })
            {
                object a = New(cell), b = New(clone);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                Array left = length < 0 ? null : name == "WriteSignedBits"
                    ? new[] { int.MinValue, -1, int.MaxValue }.Take(length).ToArray()
                    : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                Array right = (Array)left?.Clone();
                Check(recovered, methods.Values.Single(m => m.Name == name), a, b,
                    new object[] { left, index, value }, new object[] { right, index, value },
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
                Check(recovered, methods.Values.Single(m => m.Name == "WriteAfterRetarget"), a, b,
                    new object[] { al, index, value }, new object[] { ar, index, value }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right); SameArray(al, ar);
                        bool valid = length >= 0 && index >= 0 && index < length;
                        if (!ReferenceEquals(Get(a, "Values"), valid ? al : left)
                            || !ReferenceEquals(Get(b, "Values"), valid ? ar : right))
                            throw new Exception("Indirect write lost the addressed array identity");
                    });
            }
            foreach (int otherKind in new[] { 0, 1, 2 })
            {
                object a = New(cell), b = New(clone);
                object oa = otherKind == 0 ? null : otherKind == 1 ? a : New(cell);
                object ob = otherKind == 0 ? null : otherKind == 1 ? b : New(clone);
                Set(a, "Effects", 7); Set(b, "Effects", 7);
                if (otherKind == 2) { Set(oa, "Effects", int.MaxValue); Set(ob, "Effects", int.MaxValue); }
                uint[] left = length < 0 ? null : new[] { 0x80000000u, uint.MaxValue, 1u }.Take(length).ToArray();
                uint[] right = (uint[])left?.Clone();
                Check(recovered, methods.Values.Single(m => m.Name == "WriteAfterCall"), a, b,
                    new object[] { left, index, value, oa }, new object[] { right, index, value, ob }, () => {
                        SameFields(a, b, "Effects"); SameArray(left, right);
                        if (oa != null) SameFields(oa, ob, "Effects");
                    });
            }
        }
        foreach (string name in new[] { "Take", "TakeSigned", "Duplicate", "Retarget", "ReadUnsigned", "ReadSignedBits", "ReadAfterRetarget", "ReadSnapshot", "ReadAfterWrite", "WriteUnsigned", "WriteSignedBits", "UpdateViaDup", "WriteAfterRetarget", "WriteAfterCall" })
        {
            object[] values = name == "WriteAfterCall" ? new object[] { null, 0, 1u, null }
                : methods.Values.Single(m => m.Name == name).GetParameters().Length == 3
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
            "Take", "TakeSigned", "Duplicate", "Retarget", "Bump", "ReadUnsigned", "ReadSignedBits", "ReadSnapshot", "ReadAfterWrite", "ReadAfterRetarget",
            "WriteUnsigned", "WriteSignedBits", "UpdateViaDup", "WriteAfterRetarget", "WriteAfterCall"
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
