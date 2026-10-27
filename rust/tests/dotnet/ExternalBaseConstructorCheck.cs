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
using System.Reflection;
using System.Reflection.Emit;
using System.Runtime.CompilerServices;
using RelationLibrary;

internal static class ExternalBaseConstructorBehavior
{
    static Action<TReceiver, int> OriginalSeed<TReceiver>()
    {
        ConstructorInfo constructor = typeof(TReceiver).GetConstructor(new[] { typeof(int) })
            ?? throw new Exception("Owned constructor missing");
        var method = new DynamicMethod("OriginalSeed", typeof(void),
            new[] { typeof(TReceiver), typeof(int) }, typeof(ExternalBaseConstructorBehavior).Module, true);
        ILGenerator il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Call, constructor);
        il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<TReceiver, int>>();
    }

    static Action<TReceiver, TValue, TKey> OriginalTyped<TReceiver, TKey, TValue>()
    {
        ConstructorInfo constructor = typeof(TReceiver).GetConstructor(new[] { typeof(TValue), typeof(TKey) })
            ?? throw new Exception("Owned typed constructor missing");
        var method = new DynamicMethod("OriginalTyped", typeof(void),
            new[] { typeof(TReceiver), typeof(TValue), typeof(TKey) }, typeof(ExternalBaseConstructorBehavior).Module, true);
        ILGenerator il = method.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Ldarg_2);
        il.Emit(OpCodes.Call, constructor);
        il.Emit(OpCodes.Ret);
        return method.CreateDelegate<Action<TReceiver, TValue, TKey>>();
    }

    static (string Error, string Trace) Observe(Action action)
    {
        ConstructorTrace.Log = "";
        string error = "";
        try { action(); }
        catch (Exception exception) { error = exception.GetType().FullName + ":" + exception.Message; }
        return (error, ConstructorTrace.Log);
    }

    static int Seeds<TKey, TValue>()
    {
        var original = OriginalSeed<RelationCases.ForeignOuter<TKey, TValue>.NestedForeign>();
        int count = 0;
        foreach (int seed in new[] { 0, 9, -17, int.MaxValue })
        {
            var before = (RelationCases.ForeignOuter<TKey, TValue>.NestedForeign)RuntimeHelpers.GetUninitializedObject(
                typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedForeign));
            var after = (RelationCases.ForeignOuter<TKey, TValue>.NestedForeign)RuntimeHelpers.GetUninitializedObject(
                typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedForeign));
            var expected = Observe(() => original(before, seed));
            var actual = Observe(() => Recovered<TKey, TValue>.RunSeed(after, seed));
            string expectedError = seed == -17 ? "System.InvalidOperationException:owned constructor failure" : "";
            if (expected.Error != expectedError || expected.Trace.Length == 0)
                throw new Exception("Owned seed constructor did not execute as expected: " + expected.Error);
            if (expected != actual || before.Seed != after.Seed || before.Derived != after.Derived)
                throw new Exception("Recovered external constructor differs: seed " + seed);
            count++;
        }
        return count;
    }

    static int Typed<TKey, TValue>(TValue first, TKey second)
    {
        var original = OriginalTyped<RelationCases.ForeignOuter<TKey, TValue>.NestedTyped, TKey, TValue>();
        var before = (RelationCases.ForeignOuter<TKey, TValue>.NestedTyped)RuntimeHelpers.GetUninitializedObject(
            typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedTyped));
        var after = (RelationCases.ForeignOuter<TKey, TValue>.NestedTyped)RuntimeHelpers.GetUninitializedObject(
            typeof(RelationCases.ForeignOuter<TKey, TValue>.NestedTyped));
        var expected = Observe(() => original(before, first, second));
        var actual = Observe(() => Recovered<TKey, TValue>.RunTyped(after, first, second));
        if (expected.Error.Length != 0 || expected.Trace.Length == 0)
            throw new Exception("Owned typed constructor did not execute as expected: " + expected.Error);
        if (expected != actual
            || !Equals(before.First, after.First) || !Equals(before.Second, after.Second)
            || (!typeof(TValue).IsValueType && !ReferenceEquals(before.First, after.First))
            || (!typeof(TKey).IsValueType && !ReferenceEquals(before.Second, after.Second)))
            throw new Exception("Recovered external constructor differs: typed arguments");
        return 1;
    }

    static void Main()
    {
        object first = new object();
        object second = new object();
        int comparisons = Typed<object, object>(first, second);
        comparisons += Typed<int, string>("owned", 23);
        comparisons += Typed<string, int>(41, "other");
        comparisons += Typed<object, string>(null, second);
        comparisons += Seeds<int, string>();
        comparisons += Seeds<string, int>();
        Console.WriteLine($"{comparisons} comparisons; exact base constructor, arguments, receiver state, trace and exceptions match");
    }
}
