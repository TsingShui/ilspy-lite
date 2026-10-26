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

namespace StructReceiverCases;

public struct Point
{
    public int X;
    public int Y;
    public long W;
    public object Tag;
    public static int Writes;
    public Point(int x, int y, long w, object tag) { X = x; Y = y; W = w; Tag = tag; }
    public int Read() => X;
    public int Sum() => X + Y;
    public long Wide() => W;
    public object Ref() => Tag;
    public void SetX(int value) { X = value; Writes++; }
    public int Bump(int delta) { int old = X; X = old + delta; return old; }
    public int SwapX(Point other) { int value = X; X = other.X; return value; }
}

public struct Line
{
    public Point A;
    public Point B;
    public int ReadNested() => A.X + B.Y;
    public void Move(int delta) { A.X += delta; }
    public long WideNested() => A.W + B.W;
}

public static class Ops
{
    public static int LocalWrite(int seed)
    {
        Point point = default;
        point.X = seed;
        point.Y = seed * 2;
        return point.X + point.Y;
    }

    public static long LocalWide(long seed)
    {
        Line line = default;
        line.A.W = seed;
        line.B.W = seed + 1;
        return line.WideNested();
    }

    public static int LocalCall(int seed)
    {
        Point point = default;
        point.SetX(seed);
        return point.Read();
    }

    public static int ArgumentRead(Point value) => value.Sum();
    public static void ArgumentWrite(Point value, int next) => value.SetX(next);
    public static int NestedArgument(Line value) => value.ReadNested();

    public static Point UnsupportedConstruct() => new Point(1, 2, 3, null);
    public static object UnsupportedBox(Point value) => value;
    public static int UnsupportedByRef(ref Point value) => value.X;
    public static int UnsupportedLocalEscape(Point value) => Helper(ref value);
    public static ref long UnsupportedRefReturn(ref Line line) => ref line.B.W;
    public static int UnsupportedArray(Point[] values) => values[0].X;

    static int Helper(ref Point value) => value.X;
}
