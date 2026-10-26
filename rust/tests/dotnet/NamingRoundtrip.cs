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
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;

// Test-only ABI discovery and direct calls. Recovered state never comes from
// an original factory, receiver, or method result.
internal static class NamingRoundtrip
{
    const BindingFlags Members = BindingFlags.Public | BindingFlags.NonPublic |
        BindingFlags.Instance | BindingFlags.DeclaredOnly;
    static readonly int[] Boundaries = { int.MinValue, int.MinValue + 1, -1, 0, 1, int.MaxValue - 1, int.MaxValue };
    static readonly Dictionary<short, OpCode> Opcodes = typeof(OpCodes).GetFields()
        .Where(f => f.FieldType == typeof(OpCode))
        .Select(f => (OpCode)f.GetValue(null)!).ToDictionary(o => o.Value);

    sealed class StateMismatch : Exception
    {
        public string Kind { get; }
        public StateMismatch(string kind, string message) : base(message) { Kind = kind; }
    }

    sealed class Side
    {
        public readonly Type Type;
        public readonly FieldInfo Value;
        public readonly ConstructorInfo Constructor;
        public readonly MethodInfo Method;
        public readonly Action<object> Construct;
        public readonly Func<object, int, int> Invoke;

        public Side(Assembly assembly, bool recovered)
        {
            string owner = recovered ? Alias("<>c__DisplayClass0_0") : "<>c__DisplayClass0_0";
            Type = assembly.GetType("NamingCases.Factory+" + owner, throwOnError: true)!;
            Require(Type.IsNestedPrivate && Type.IsSealed && Type.BaseType == typeof(object), "closure declaration ABI");
            Require(Type.GetFields(Members).Length == 1, "exactly one captured field");
            Value = Type.GetField("value", Members)!;
            Require(Value != null && Value.FieldType == typeof(int) && !Value.IsStatic, "captured I4 field ABI");
            Constructor = Type.GetConstructor(Members, null, Type.EmptyTypes, null)!;
            Require(Constructor != null && Constructor.IsPublic && Type.GetConstructors(Members).Length == 1,
                "single public parameterless instance constructor");
            string member = recovered ? "AccumulatorLambda0" : "<Accumulator>b__0";
            Method = Type.GetMethod(member, Members)!;
            Require(Method != null && !Method.IsStatic && !Method.IsGenericMethod && Method.IsAssembly &&
                Method.ReturnType == typeof(int) && Method.GetParameters().Select(p => p.ParameterType).SequenceEqual(new[] { typeof(int) }) &&
                Type.GetMethods(Members).Length == 1, "single internal instance I4 method ABI");
            VerifyBodies(this);
            var ctor = new DynamicMethod("direct_constructor", typeof(void), new[] { typeof(object) }, typeof(NamingRoundtrip).Module, true);
            ILGenerator il = ctor.GetILGenerator();
            il.Emit(OpCodes.Ldarg_0);
            il.Emit(OpCodes.Castclass, Type);
            il.Emit(OpCodes.Call, Constructor);
            il.Emit(OpCodes.Ret);
            Construct = ctor.CreateDelegate<Action<object>>();
            var method = new DynamicMethod("direct_method", typeof(int), new[] { typeof(object), typeof(int) }, typeof(NamingRoundtrip).Module, true);
            il = method.GetILGenerator();
            il.Emit(OpCodes.Ldarg_0);
            il.Emit(OpCodes.Castclass, Type);
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(OpCodes.Call, Method);
            il.Emit(OpCodes.Ret);
            Invoke = method.CreateDelegate<Func<object, int, int>>();
        }

        public object Receiver(int value)
        {
            object receiver = RuntimeHelpers.GetUninitializedObject(Type);
            Value.SetValue(receiver, value);
            return receiver;
        }

        public int Read(object receiver) => (int)Value.GetValue(receiver)!;
    }

    static string Alias(string raw) => "__name_" + Convert.ToHexString(System.Text.Encoding.UTF8.GetBytes(raw));

    static void Require(bool condition, string description)
    {
        if (!condition) throw new Exception("Invalid gate input: " + description);
    }

    // Decode every operand rather than treating token-like byte sequences as calls.
    static List<(OpCode Code, object Operand)> Instructions(MethodBase method)
    {
        MethodBody body = method.GetMethodBody()!;
        Require(body != null && body.ExceptionHandlingClauses.Count == 0, "ordinary body without EH");
        byte[] bytes = body.GetILAsByteArray()!;
        var instructions = new List<(OpCode, object)>();
        int offset = 0;
        while (offset < bytes.Length)
        {
            short code = bytes[offset++];
            if (code == 0xFE) code = (short)(0xFE00 | bytes[offset++]);
            Require(Opcodes.ContainsKey(code), "known IL opcode");
            OpCode op = Opcodes[code];
            object operand = null;
            int size = op.OperandType switch {
                OperandType.InlineNone => 0,
                OperandType.ShortInlineBrTarget or OperandType.ShortInlineI or OperandType.ShortInlineVar => 1,
                OperandType.InlineVar => 2,
                OperandType.InlineI8 or OperandType.InlineR => 8,
                OperandType.InlineSwitch => throw new Exception("Unexpected switch in bounded closure"),
                _ => 4
            };
            Require(offset + size <= bytes.Length, "complete IL operand");
            if (op.OperandType == OperandType.InlineMethod || op.OperandType == OperandType.InlineField ||
                op.OperandType == OperandType.InlineType || op.OperandType == OperandType.InlineTok)
                operand = method.Module.ResolveMember(BitConverter.ToInt32(bytes, offset))!;
            if (op.OperandType == OperandType.ShortInlineBrTarget)
                operand = (sbyte)bytes[offset];
            offset += size;
            instructions.Add((op, operand));
        }
        return instructions;
    }

    static void VerifyBodies(Side side)
    {
        var ctor = Instructions(side.Constructor).Where(i => i.Code != OpCodes.Nop).ToArray();
        // The nonoptimized C# explicit return emits br.s 0 immediately before ret.
        bool returnShape = ctor.Length == 3 && ctor[2].Code == OpCodes.Ret ||
            ctor.Length == 4 && ctor[2].Code == OpCodes.Br_S && ctor[2].Operand is sbyte displacement &&
            displacement == 0 && ctor[3].Code == OpCodes.Ret;
        Require(returnShape && ctor[0].Code == OpCodes.Ldarg_0 && ctor[1].Code == OpCodes.Call,
            "exact ldarg.0/call/ret Object constructor shape (nops and br.s 0 allowed)");
        Require(ctor[1].Operand is ConstructorInfo target && target.DeclaringType == typeof(object) &&
            target.GetParameters().Length == 0 && !target.IsStatic, "exact core Object constructor target");
        foreach (var instruction in Instructions(side.Method))
        {
            Require(instruction.Code.OperandType != OperandType.InlineMethod &&
                instruction.Code.OperandType != OperandType.InlineSig && instruction.Code != OpCodes.Jmp,
                "captured-I4 body has no calls, function pointers, or original-selected proxy");
            if (instruction.Operand is MemberInfo member)
                Require(member is FieldInfo field && field == side.Value, "only the selected captured field is referenced");
        }
    }

    static void Compare(int actual, int expected, string kind, string context)
    {
        if (actual != expected) throw new StateMismatch(kind, $"{context}: expected {expected}, actual {actual}");
    }

    static void NullCall(Side side, int delta)
    {
        try { side.Invoke(null, delta); }
        catch (NullReferenceException) { return; }
        throw new Exception("Null receiver must throw exactly NullReferenceException");
    }

    static void Main(string[] args)
    {
        try
        {
            Require(args.Length == 2, "original and recovered assembly paths");
            Assembly original = new AssemblyLoadContext("original", true).LoadFromAssemblyPath(args[0]);
            Assembly recovered = new AssemblyLoadContext("recovered", true).LoadFromAssemblyPath(args[1]);
            Require(recovered.GetReferencedAssemblies().All(a => a.Name == "System.Runtime" || a.Name == "System.Private.CoreLib"),
                "recovered assembly references only core framework, never the original assembly");
            Require(recovered.GetReferencedAssemblies().All(a => a.Name != original.GetName().Name), "no original assembly reference");
            Require(recovered.GetTypes().Length == 2, "only original ordinary parent and selected nested class");
            var reference = new Side(original, false);
            var actual = new Side(recovered, true);
            int constructors = 0, methods = 0, nulls = 0;
            // One fresh zero-initialized receiver plus seven independently preset states.
            foreach (int? initial in new int?[] { null }.Concat(Boundaries.Select(i => (int?)i)))
            {
                object left = initial.HasValue ? reference.Receiver(initial.Value) : RuntimeHelpers.GetUninitializedObject(reference.Type);
                object right = initial.HasValue ? actual.Receiver(initial.Value) : RuntimeHelpers.GetUninitializedObject(actual.Type);
                for (int repeat = 0; repeat < 3; repeat++)
                {
                    reference.Construct(left);
                    actual.Construct(right);
                    Compare(reference.Read(left), initial ?? 0, "reference-ctor-state", $"initial={initial}, repeat={repeat}");
                    Compare(actual.Read(right), initial ?? 0, "ctor-state", $"initial={initial}, repeat={repeat}");
                    constructors++;
                }
            }
            foreach (int initial in Boundaries)
                foreach (int delta in Boundaries)
                {
                    object left = reference.Receiver(initial);
                    object right = actual.Receiver(initial);
                    int expected = initial;
                    for (int repeat = 0; repeat < 3; repeat++)
                    {
                        string context = $"initial={initial}, delta={delta}, repeat={repeat}";
                        Compare(reference.Invoke(left, delta), expected, "reference-old-return", context);
                        Compare(actual.Invoke(right, delta), expected, "old-return", context);
                        expected = unchecked(expected + delta);
                        Compare(reference.Read(left), expected, "reference-field-write", context);
                        Compare(actual.Read(right), expected, "field-write", context);
                        methods++;
                    }
                }
            foreach (int delta in Boundaries)
            {
                NullCall(reference, delta);
                NullCall(actual, delta);
                nulls++;
            }
            Console.WriteLine(JsonSerializer.Serialize(new {
                status = "passed", constructorPairs = constructors, methodPairs = methods, nullPairs = nulls,
                totalPairs = constructors + methods + nulls, coreObjectShapes = 2, noOriginalProxy = true
            }));
        }
        catch (StateMismatch failure)
        {
            Console.WriteLine(JsonSerializer.Serialize(new { status = "behavior-mismatch", kind = failure.Kind, detail = failure.Message }));
            Environment.ExitCode = 42;
        }
    }
}
