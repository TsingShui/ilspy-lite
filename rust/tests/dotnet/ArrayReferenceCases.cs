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

namespace ArrayReferenceCases;

public struct Point
{
    public int X;
    public object Tag;
}

public enum Kind : long { None, High = long.MaxValue }

public class Cell
{
    public byte[] Values;
    public static byte[][] Cache;
    public byte[] Exchange(byte[] next)
    {
        byte[] before = Values;
        Values = next;
        return before;
    }
}

public static class Ops
{
    public static byte[] LocalIdentity(byte[] value) => value;
    public static string[] StringIdentity(string[] value) => value;
    public static object[] ExternalIdentity(object[] value) => value;
    public static byte[][] NestedIdentity(byte[][] value) => value;
    public static Point[] ValueArgument(Point[] value) => value;
    public static List<int>[] ArrayArgument(List<int>[] value) => value;
    public static Kind[] EnumIdentity(Kind[] value) => value;
    public static char[] CharIdentity(char[] value) => value;
    public static double[] FloatIdentity(double[] value) => value;
    public static long[] LongIdentity(long[] value) => value;
    public static nint[] NativeIdentity(nint[] value) => value;
    public static ulong[] UnsignedIdentity(ulong[] value) => value;
    public static byte[] Null() => null;
    public static byte[] Choose(bool first, byte[] left, byte[] right) => first ? left : right;
    public static object Upcast(byte[] value) => value;
    public static byte[] Snapshot(byte[] value)
    {
        byte[] saved = value;
        value = null;
        return saved;
    }
    public static byte[] Forward(byte[] value) => LocalIdentity(value);
    public static byte[] ReadField(Cell cell) => cell.Values;
    public static byte[] ExchangeField(Cell cell, byte[] next)
    {
        byte[] before = cell.Values;
        cell.Values = next;
        return before;
    }
    public static byte[][] StaticExchange(byte[][] next)
    {
        byte[][] before = Cell.Cache;
        Cell.Cache = next;
        return before;
    }
    public static object[] UnsupportedCovariance(string[] value) => value;
    public static object UnsupportedRead(object[] value) => value[0];
    public static void UnsupportedStore(object[] value, object next) => value[0] = next;
    public static byte[] UnsupportedAllocation(int length) => new byte[length];
    public static Point UnsupportedValue(Point[] value) => value[0];
    public static int UnsupportedLength(byte[] value) => value.Length;
    public static T[] UnsupportedOpen<T>(T[] value) => value;
}
