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
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.Json;
using OpenRA.Mods.Common.Traits;

// Test-only same-receiver constructor calls; selected original bodies are oracles only.
public static class FrozenConstructors
{
    public sealed class CallbackReached : Exception { }
    const BindingFlags Fields = BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.DeclaredOnly;
    static Module common;
    static ConstructorInfo coreCtor;
    static ConstructorInfo baseCtor;
    static Action<object, object[]> coreCall;
    static Action<object, object[]> baseCall;
    static object expectedReceiver;
    static object[] expectedArguments;
    static object expectedDerivedInfo;
    static int leafCount;
    static int caseCount;
    static Dictionary<string, string> ilEvidence;

    static void Require(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
    }

    static object Fresh(Type type) => RuntimeHelpers.GetUninitializedObject(type);

    static FieldInfo Info(Type type) => type.GetField("Info", Fields)
        ?? throw new InvalidOperationException("Missing declared Info field");

    static void PinIL(MethodBase method)
    {
        Require(method.GetMethodBody() is { } body && body.ExceptionHandlingClauses.Count == 0
            && Convert.ToHexString(body.GetILAsByteArray()) == ilEvidence[method.MetadataToken.ToString("X8")],
            "CLI/reflection IL evidence mismatch: " + method);
    }

    record Instruction(OpCode Code, MemberInfo Target);

    static Instruction[] Protocol(MethodBase method, params OpCode[] expected)
    {
        PinIL(method);
        var codes = typeof(OpCodes).GetFields(BindingFlags.Public | BindingFlags.Static)
            .Where(f => f.FieldType == typeof(OpCode)).Select(f => (OpCode)f.GetValue(null))
            .ToDictionary(c => unchecked((ushort)c.Value));
        var bytes = method.GetMethodBody().GetILAsByteArray();
        var instructions = new List<Instruction>();
        for (int offset = 0; offset < bytes.Length;)
        {
            ushort value = bytes[offset++];
            if (value == 0xFE)
                value = (ushort)(0xFE00 | bytes[offset++]);
            var code = codes[value];
            MemberInfo target = null;
            if (code.OperandType == OperandType.InlineMethod || code.OperandType == OperandType.InlineField)
            {
                target = common.ResolveMember(BitConverter.ToInt32(bytes, offset));
                offset += 4;
            }
            else
                Require(code.OperandType == OperandType.InlineNone, "Unexpected constructor protocol operand");
            instructions.Add(new Instruction(code, target));
        }
        Require(instructions.Select(i => i.Code).SequenceEqual(expected), "Constructor opcode protocol drift: " + method);
        return instructions.ToArray();
    }

    static ConstructorInfo Constructor(int token, Type owner, params Type[] arguments)
    {
        var ctor = common.ResolveMethod(token) as ConstructorInfo;
        Require(ctor != null && ctor.DeclaringType == owner && !ctor.IsStatic
            && ctor.CallingConvention == (CallingConventions.Standard | CallingConventions.HasThis)
            && ctor.GetParameters().Select(p => p.ParameterType).SequenceEqual(arguments),
            "Constructor token/full signature drift: " + token.ToString("X8"));
        return ctor;
    }

    // This is an independent exact-call leaf, not a decompilation of its target.
    static Action<object, object[]> Direct(ConstructorInfo ctor)
    {
        var method = new DynamicMethod("ExactCtor_" + ctor.MetadataToken.ToString("X8"),
            typeof(void), new[] { typeof(object), typeof(object[]) }, typeof(FrozenConstructors).Module, true);
        var il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Castclass, ctor.DeclaringType);
        int index = 0;
        foreach (var parameter in ctor.GetParameters())
        {
            il.Emit(OpCodes.Ldarg_1);
            il.Emit(OpCodes.Ldc_I4, index++);
            il.Emit(OpCodes.Ldelem_Ref);
            il.Emit(OpCodes.Castclass, parameter.ParameterType);
        }
        il.Emit(OpCodes.Call, ctor);
        il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<object, object[]>>();
    }

    public static void Leaf(object receiver, int token, object[] arguments)
    {
        Require(++leafCount == 1 && ReferenceEquals(receiver, expectedReceiver), "Leaf THIS/count");
        Require(arguments.Length == expectedArguments.Length
            && arguments.Zip(expectedArguments).All(p => ReferenceEquals(p.First, p.Second)), "Leaf argument identity/order");
        Require(ReferenceEquals(Info(receiver.GetType()).GetValue(receiver), expectedDerivedInfo),
            "Leaf order/derived Info");
        if (receiver is FrozenMustReceiver)
        {
            Require(token == 0x0A000109, "Core Object leaf token");
            coreCall(receiver, arguments);
        }
        else
        {
            Require(receiver is FrozenProximityReceiver && token == 0x06001A4F, "Immediate-base leaf token");
            baseCall(receiver, arguments);
        }
    }

    static Exception Capture(Action action)
    {
        try { action(); return null; }
        catch (Exception ex) { return ex; }
    }

    static void Prepare(object receiver, object[] arguments)
    {
        expectedReceiver = receiver;
        expectedArguments = arguments;
        expectedDerivedInfo = Info(receiver.GetType()).GetValue(receiver);
        leafCount = 0;
    }

    static void ValidateIdentities()
    {
        common = typeof(MustBeDestroyed).Module;
        Require(common.ModuleVersionId == new Guid("daac77a9-0bf7-4155-8229-f659cf68d982"), "Common MVID drift");
        Require(typeof(OpenRA.ActorInitializer).Module.ModuleVersionId == new Guid("6b57af11-2705-4259-afe6-fb984ced7168"), "Game MVID drift");
        Require(typeof(MustBeDestroyed).MetadataToken == 0x0200047E
            && typeof(ProximityCapturable).MetadataToken == 0x02000514, "Frozen TypeDef drift");
        Require(typeof(FrozenMustReceiver).BaseType == typeof(MustBeDestroyed).BaseType
            && typeof(FrozenProximityReceiver).BaseType == typeof(ProximityCapturable).BaseType, "Actual immediate base mismatch");
        Require(Info(typeof(MustBeDestroyed)).MetadataToken == 0x04001374
            && Info(typeof(ProximityCapturable)).MetadataToken == 0x040015BB
            && Info(typeof(MustBeDestroyed)).IsInitOnly && Info(typeof(ProximityCapturable)).IsInitOnly,
            "Original readonly field identity drift");
        Require(!Info(typeof(FrozenMustReceiver)).IsInitOnly && !Info(typeof(FrozenProximityReceiver)).IsInitOnly
            && Info(typeof(FrozenMustReceiver)).FieldType == typeof(MustBeDestroyedInfo)
            && Info(typeof(FrozenProximityReceiver)).FieldType == typeof(ProximityCapturableInfo), "Test-only field binding drift");
        coreCtor = (ConstructorInfo)common.ResolveMethod(0x0A000109);
        Require(coreCtor.DeclaringType == typeof(object) && coreCtor.GetParameters().Length == 0, "Core Object constructor identity");
        coreCall = Direct(coreCtor);
        baseCtor = Constructor(0x06001A4F, typeof(ProximityCapturableBase), typeof(OpenRA.ActorInitializer), typeof(ProximityCapturableBaseInfo));
        Require(baseCtor.IsFamily, "Immediate base accessibility drift");
        baseCall = Direct(baseCtor);
        var must = Protocol(Constructor(0x060017D4, typeof(MustBeDestroyed), typeof(MustBeDestroyedInfo)),
            OpCodes.Ldarg_0, OpCodes.Call, OpCodes.Ldarg_0, OpCodes.Ldarg_1, OpCodes.Stfld, OpCodes.Ret);
        Require(must[1].Target == coreCtor && must[4].Target == Info(typeof(MustBeDestroyed)), "Must original call/write protocol");
        var proximity = Protocol(Constructor(0x06001A46, typeof(ProximityCapturable), typeof(OpenRA.ActorInitializer), typeof(ProximityCapturableInfo)),
            OpCodes.Ldarg_0, OpCodes.Ldarg_1, OpCodes.Ldarg_2, OpCodes.Call,
            OpCodes.Ldarg_0, OpCodes.Ldarg_2, OpCodes.Stfld, OpCodes.Ret);
        Require(proximity[3].Target == baseCtor && proximity[6].Target == Info(typeof(ProximityCapturable)), "Proximity original call/write protocol");
        // The null init dereference precedes Self assignment and Actor.Owner.
        // Resolve every pre-fault operation before permitting any case to execute.
        var prefix = Protocol(baseCtor, OpCodes.Ldarg_0, OpCodes.Newobj, OpCodes.Stfld,
            OpCodes.Ldarg_0, OpCodes.Call, OpCodes.Ldarg_0, OpCodes.Ldarg_2, OpCodes.Stfld,
            OpCodes.Ldarg_0, OpCodes.Ldarg_1, OpCodes.Ldfld, OpCodes.Stfld,
            OpCodes.Ldarg_0, OpCodes.Ldarg_0, OpCodes.Ldfld, OpCodes.Callvirt, OpCodes.Stfld, OpCodes.Ret);
        var listCtor = (ConstructorInfo)common.ResolveMethod(0x0A000273);
        Require(listCtor.DeclaringType == typeof(System.Collections.Generic.List<OpenRA.Actor>)
            && listCtor.GetParameters().Length == 0, "Pre-fault allocation target drift");
        var initSelf = common.ResolveField(0x0A00071E);
        Require(initSelf.DeclaringType == typeof(OpenRA.ActorInitializer) && initSelf.Name == "Self"
            && !initSelf.IsStatic && initSelf.FieldType == typeof(OpenRA.Actor), "Null-init fault operand drift");
        Require(prefix[1].Target == listCtor && prefix[2].Target == common.ResolveField(0x040015C4)
            && prefix[4].Target == coreCtor && prefix[7].Target == Info(typeof(ProximityCapturableBase))
            && prefix[10].Target == initSelf, "Safe null-init prefix operands drift");
        foreach (var (token, name, type) in new[] {
            (0x040015C4, "actorsInRange", typeof(System.Collections.Generic.List<OpenRA.Actor>)),
            (0x040015C2, "Info", typeof(ProximityCapturableBaseInfo)),
            (0x040015C3, "Self", typeof(OpenRA.Actor)), (0x040015C1, "OriginalOwner", typeof(OpenRA.Player)) })
        {
            var field = common.ResolveField(token);
            Require(field.DeclaringType == typeof(ProximityCapturableBase) && field.Name == name
                && field.FieldType == type && !field.IsStatic, "Base field operand drift");
        }
        Console.WriteLine("Pinned MVIDs, TypeDefs, full constructor signatures, fields, exact selected/base IL verified");
    }

    static void MustCases()
    {
        var original = Direct(Constructor(0x060017D4, typeof(MustBeDestroyed), typeof(MustBeDestroyedInfo)));
        foreach (bool dirty in new[] { false, true })
        {
            var left = (MustBeDestroyed)Fresh(typeof(MustBeDestroyed));
            var right = (FrozenMustReceiver)Fresh(typeof(FrozenMustReceiver));
            if (dirty)
            {
                Info(left.GetType()).SetValue(left, Fresh(typeof(MustBeDestroyedInfo)));
                right.Info = (MustBeDestroyedInfo)Fresh(typeof(MustBeDestroyedInfo));
            }
            // Repeated overwrite covers fresh/dirty, null, and distinct opaque identities.
            int step = 0;
            foreach (int value in new[] { 1, 2, 0, 1, 0 })
            {
                var leftInfo = value == 0 ? null : (MustBeDestroyedInfo)Fresh(typeof(MustBeDestroyedInfo));
                var rightInfo = value == 0 ? null : (MustBeDestroyedInfo)Fresh(typeof(MustBeDestroyedInfo));
                Prepare(right, Array.Empty<object>());
                var leftException = Capture(() => original(left, new object[] { leftInfo }));
                var rightException = Capture(() => FrozenMustReceiverBody.Run(right, rightInfo));
                Require(leftException == null && rightException == null, "Must success exception mismatch: " + rightException);
                Require(ReferenceEquals(left.Info, leftInfo) && ReferenceEquals(right.Info, rightInfo), "Must Info identity");
                Require(leafCount == 1, "Must leaf count");
                ++caseCount;
                Console.WriteLine($"Must success initially-dirty={dirty} overwrite-step={step++} info={(value == 0 ? "null" : "opaque-fresh")} exact-call=1 identity=passed");
            }
        }
    }

    static void ProximityCases()
    {
        var original = Direct(Constructor(0x06001A46, typeof(ProximityCapturable), typeof(OpenRA.ActorInitializer), typeof(ProximityCapturableInfo)));
        var baseFields = typeof(ProximityCapturableBase).GetFields(Fields);
        var baseInfo = Info(typeof(ProximityCapturableBase));
        var listField = common.ResolveField(0x040015C4);
        foreach (bool dirty in new[] { false, true })
        foreach (bool nullInfo in new[] { false, true })
        {
            var left = (ProximityCapturable)Fresh(typeof(ProximityCapturable));
            var right = (FrozenProximityReceiver)Fresh(typeof(FrozenProximityReceiver));
            // Initialize each receiver independently, never from an original poststate.
            foreach (object receiver in new object[] { left, right })
            {
                if (dirty)
                {
                    Info(receiver.GetType()).SetValue(receiver, Fresh(typeof(ProximityCapturableInfo)));
                    baseInfo.SetValue(receiver, Fresh(typeof(ProximityCapturableInfo)));
                    listField.SetValue(receiver, new System.Collections.Generic.List<OpenRA.Actor> { null });
                    typeof(ProximityCapturableBase).GetField("trigger", Fields).SetValue(receiver, 17);
                    typeof(ProximityCapturableBase).GetField("skipTriggerUpdate", Fields).SetValue(receiver, true);
                }
            }
            var leftBefore = baseFields.Select(f => f.GetValue(left)).ToArray();
            var rightBefore = baseFields.Select(f => f.GetValue(right)).ToArray();
            var leftDerived = left.Info;
            var rightDerived = right.Info;
            var leftInfo = nullInfo ? null : (ProximityCapturableInfo)Fresh(typeof(ProximityCapturableInfo));
            var rightInfo = nullInfo ? null : (ProximityCapturableInfo)Fresh(typeof(ProximityCapturableInfo));
            Prepare(right, new object[] { null, rightInfo });
            var leftException = Capture(() => original(left, new object[] { null, leftInfo }));
            var rightException = Capture(() => FrozenProximityReceiverBody.Run(right, null, rightInfo));
            Require(leftException is NullReferenceException && rightException is NullReferenceException
                && leftException.Message == rightException.Message, "Proximity bounded fault mismatch: " + rightException);
            Require(leafCount == 1 && ReferenceEquals(left.Info, leftDerived)
                && ReferenceEquals(right.Info, rightDerived), "Proximity fault leaf count/derived Info preservation");
            Require(ReferenceEquals(baseInfo.GetValue(left), leftInfo) && ReferenceEquals(baseInfo.GetValue(right), rightInfo), "Proximity partial base Info identity");
            var leftList = (IList)listField.GetValue(left);
            var rightList = (IList)listField.GetValue(right);
            Require(leftList != null && rightList != null && leftList.Count == 0 && rightList.Count == 0
                && !ReferenceEquals(leftList, rightList)
                && !ReferenceEquals(leftList, leftBefore[Array.IndexOf(baseFields, listField)])
                && !ReferenceEquals(rightList, rightBefore[Array.IndexOf(baseFields, listField)]), "Proximity fresh partial list");
            for (int i = 0; i < baseFields.Length; i++)
            {
                var field = baseFields[i];
                if (field == baseInfo || field == listField)
                    continue;
                Require(Equals(field.GetValue(left), leftBefore[i]) && Equals(field.GetValue(right), rightBefore[i])
                    && Equals(field.GetValue(left), field.GetValue(right)), "Proximity untouched base field: " + field.Name);
            }
            ++caseCount;
            Console.WriteLine($"Proximity exception-path-only init=null info={(nullInfo ? "null" : "opaque-fresh")} dirty={dirty} NullReferenceException partial-base/derived-state=passed exact-call=1 callbacks=0");
        }
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    static void Execute()
    {
        ValidateIdentities();
        MustCases();
        ProximityCases();
        Console.WriteLine($"PASS cases={caseCount} Must-success=10 Proximity-exception-only=4; no game execution/success-domain claim");
    }

    public static int Main(string[] args)
    {
        try
        {
            Require(args.Length == 2, "Expected existing corpus directory and ignored IL evidence");
            string corpus = Path.GetFullPath(args[0]);
            ilEvidence = JsonSerializer.Deserialize<Dictionary<string, string>>(File.ReadAllText(args[1]));
            AssemblyLoadContext.Default.Resolving += (context, name) => {
                foreach (string directory in new[] { "assemblies", "runtime-deps" })
                {
                    string path = Path.Combine(corpus, directory, name.Name + ".dll");
                    if (File.Exists(path))
                        return context.LoadFromAssemblyPath(path);
                }
                return null;
            };
            Execute();
            return 0;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine(ex);
            return 1;
        }
    }
}
