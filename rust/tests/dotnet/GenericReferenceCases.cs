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

namespace GenericReferenceCases;

public sealed class Box<T>
{
    public T Value;
}

public class Cell
{
    public List<int> Values;
    public static Dictionary<string, List<int>> Cache;
    public List<int> Exchange(List<int> next)
    {
        List<int> before = Values;
        Values = next;
        return before;
    }
}

public static class Ops
{
    public static Box<int> LocalIdentity(Box<int> value) => value;
    public static Box<string> StringIdentity(Box<string> value) => value;
    public static List<int> ExternalIdentity(List<int> value) => value;
    public static Dictionary<string, List<int>> NestedIdentity(Dictionary<string, List<int>> value) => value;
    public static List<int?> ValueArgument(List<int?> value) => value;
    public static List<int[]> ArrayArgument(List<int[]> value) => value;
    public static List<int> Null() => null;
    public static List<int> Choose(bool first, List<int> left, List<int> right) => first ? left : right;
    public static object Upcast(List<int> value) => value;
    public static List<int> Snapshot(List<int> value)
    {
        List<int> saved = value;
        value = null;
        return saved;
    }
    public static Box<int> Forward(Box<int> value) => LocalIdentity(value);
    public static List<int> ReadField(Cell cell) => cell.Values;
    public static List<int> ExchangeField(Cell cell, List<int> next)
    {
        List<int> before = cell.Values;
        cell.Values = next;
        return before;
    }
    public static Dictionary<string, List<int>> StaticExchange(Dictionary<string, List<int>> next)
    {
        Dictionary<string, List<int>> before = Cell.Cache;
        Cell.Cache = next;
        return before;
    }
    public static int UnsupportedField(Box<int> value) => value.Value;
    public static void UnsupportedDispatch(List<int> value, int next) => value.Add(next);
    public static int? UnsupportedValue(int? value) => value;
    public static T UnsupportedMethod<T>(T value) => value;
    public static IEnumerable<object> UnsupportedVariance(IEnumerable<string> value) => value;
}

public static class OpenOwner<T>
{
    public static Box<T> UnsupportedOpen(Box<T> value) => value;
}
