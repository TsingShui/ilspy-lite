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
using System.Runtime.Loader;

/// <summary>Compares owned general-array methods with their recovered bodies.</summary>
internal static class MultidimBehavior
{
    private static Assembly Load(string path) =>
        new AssemblyLoadContext(path, true).LoadFromAssemblyPath(Path.GetFullPath(path));

    private static object Clone(object value) => value switch
    {
        int[,] array => array.Clone(),
        double[,] array => array.Clone(),
        string[,] array => array.Clone(),
        _ => value,
    };

    private static string Describe(object value)
    {
        if (value is not Array array) return value?.ToString() ?? "null";
        var parts = new List<string>();
        foreach (var index in Indices(array))
            parts.Add(array.GetValue(index)?.ToString() ?? "null");
        var lengths = string.Join(",", Enumerable.Range(0, array.Rank).Select(array.GetLength));
        return "rank=" + array.Rank + "(" + lengths + ")[" + string.Join(",", parts) + "]";
    }

    private static IEnumerable<int[]> Indices(Array array)
    {
        if (array.Length == 0) yield break;
        var index = new int[array.Rank];
        while (true)
        {
            yield return (int[])index.Clone();
            int dimension = array.Rank - 1;
            while (dimension >= 0)
            {
                if (++index[dimension] < array.GetLength(dimension)) break;
                index[dimension] = 0;
                dimension--;
            }
            if (dimension < 0) yield break;
        }
    }

    private static object[] Candidates(Type type)
    {
        if (type == typeof(int[,])) return new object[] {
            null!,
            new int[,] { { 1, 2, 3 }, { 4, 5, -6 } },
            new int[1, 1],
            new int[0, 0],
        };
        if (type == typeof(double[,])) return new object[] {
            null!,
            new double[,] { { 1.5, double.NaN }, { -0.0, double.PositiveInfinity } },
            new double[1, 1],
        };
        if (type == typeof(string[,])) return new object[] {
            null!,
            new string[,] { { "a", null! }, { "", "b" } },
        };
        if (type == typeof(int)) return new object[] { -1, 0, 1, 2, 5 };
        return new object[] { null! };
    }

    /// <summary>Rotating Cartesian product keeps every candidate visible without
    /// an unbounded blow-up for wider signatures.</summary>
    private static IEnumerable<object[]> Arguments(ParameterInfo[] parameters)
    {
        var candidates = parameters.Select(p => Candidates(p.ParameterType)).ToArray();
        int count = candidates.Select(c => c.Length).Max();
        for (int round = 0; round < count; round++)
        {
            var arguments = new object[parameters.Length];
            for (int position = 0; position < parameters.Length; position++)
                arguments[position] = candidates[position][(round + position) % candidates[position].Length];
            yield return arguments;
        }
        if (parameters.Length == 0) yield return Array.Empty<object>();
    }

    private static (string, string) Invoke(MethodInfo method, object[] arguments)
    {
        var own = arguments.Select(Clone).ToArray();
        try
        {
            var result = method.Invoke(null, own);
            return ("ok", Describe(result) + "|" + string.Join(";", own.Select(Describe)));
        }
        catch (TargetInvocationException error)
        {
            return ("throw", error.InnerException!.GetType().FullName!);
        }
    }

    public static int Run(string[] arguments)
    {
        var original = Load(arguments[0]).GetType("MultidimCases.Grids")
            ?? throw new Exception("Owned general-array type missing.");
        var recovered = Load(arguments[1]).GetType("MultidimCases.Grids")
            ?? throw new Exception("Recovered general-array type missing.");
        var twins = recovered.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        int comparisons = 0;
        foreach (var twin in twins)
        {
            var method = original.GetMethod(twin.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered method has no owned twin: " + twin.Name);
            foreach (var candidate in Arguments(method.GetParameters()))
            {
                var left = Invoke(method, candidate);
                var right = Invoke(twin, candidate);
                comparisons++;
                if (left != right)
                {
                    Console.Error.WriteLine($"{method.Name}({string.Join(",", candidate.Select(Describe))}): {left} != {right}");
                    return 1;
                }
            }
        }
        Console.WriteLine($"General array behavior: {comparisons} comparisons; allocation shape, element reads/stores, null/bounds checks and results match.");
        return 0;
    }
}
