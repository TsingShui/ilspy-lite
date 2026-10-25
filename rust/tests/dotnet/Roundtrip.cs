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
using System.Runtime.Loader;

internal static class Roundtrip
{
    static int checks;

    static object Invoke(Type type, string name, params object[] args)
    {
        try
        {
            return type.GetMethod(name, BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static)!.Invoke(null, args);
        }
        catch (TargetInvocationException e)
        {
            return e.InnerException!.GetType().FullName;
        }
    }

    static void Main(string[] args)
    {
        Type expected = AssemblyLoadContext.Default.LoadFromAssemblyPath(args[0]).GetType("LiteSamples.ControlFlowSample")!;
        Type actual = typeof(LiteSamples.ControlFlowSample);
        void Same(string method, params object[] values)
        {
            object want = Invoke(expected, method, values);
            object got = Invoke(actual, method, values);
            if (!Equals(want, got))
                throw new Exception($"{method}: expected {want}, got {got}");
            checks++;
        }
        foreach (int n in new[] { -1, 0, 1, 2, 6, 17, 100 }) Same("Sum", n);
        foreach (int n in new[] { int.MinValue, -1, 0, 1, 2, 3, 9, int.MaxValue }) Same("Select", n);
        foreach (int n in new[] { int.MinValue, -1, 0, 17, int.MaxValue }) Same("CheckedAdd", n);
        foreach (uint n in new[] { 0u, 1u, 2u, 0x80000000u, uint.MaxValue }) Same("Shift", n);
        foreach (bool first in new[] { false, true })
            foreach (int left in new[] { int.MinValue, -1, 7, int.MaxValue })
                foreach (int right in new[] { -9, 0, int.MaxValue }) Same("Conditional", first, left, right);
        for (int i = 0; i < 8; i++)
        {
            Same("EvaluationOrder");
            Same("PreserveOldValue");
        }
        expected = AssemblyLoadContext.Default.LoadFromAssemblyPath(args[2]).GetType("IntegerCases.Cases")!;
        actual = typeof(IntegerCases.Cases);
        object[] Domain(Type t)
        {
            if (t == typeof(int)) return new object[] { int.MinValue, -1, 0, 1, 31, 32, int.MaxValue };
            if (t == typeof(uint)) return new object[] { 0u, 1u, 0x80000000u, uint.MaxValue };
            if (t == typeof(long)) return new object[] { long.MinValue, -1L, 0L, 1L, long.MaxValue };
            if (t == typeof(ulong)) return new object[] { 0UL, 1UL, 0x8000000000000000UL, ulong.MaxValue };
            if (t == typeof(bool)) return new object[] { false, true };
            throw new Exception($"Missing test domain for {t}");
        }
        foreach (var method in expected.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly))
        {
            var parameters = method.GetParameters();
            void Enumerate(int index, object[] values)
            {
                if (index == parameters.Length)
                {
                    Same(method.Name, values);
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
        Console.WriteLine($"{args[1]}: {checks} differential executions passed");
    }
}
