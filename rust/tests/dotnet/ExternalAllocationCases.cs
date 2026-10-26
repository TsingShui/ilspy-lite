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

#if DEPENDENCY
using System;

namespace ExternalAllocations;

public class Node
{
    public static int Zero;
    public int Value;
    public Node() { Value = 7; }
    public Node(int value) { Value = value; }
}

public sealed class Sealed
{
    public int Value;
    public Sealed() { Value = 3; }
}

public class Box<T>
{
    public T Item;
    public Box(T item) { Item = item; }
}

public class Outer
{
    public class Inner
    {
        public int X;
        public Inner(int x) { X = x; }
    }
}

public class Pair
{
    public string Text;
    public int Number;
    public Pair(string text, int number) { Text = text; Number = number; }
}

public class Throws
{
    public Throws() { throw new InvalidOperationException("boom"); }
}

public struct Point
{
    public int X;
    public int Y;
    public Point(int x, int y) { X = x; Y = y; }
}
#else
using ExternalAllocations;

namespace ExternalAllocationCases;

public static class Ops
{
    // Returned rather than read: field access on a foreign type is its own slice,
    // so these methods isolate allocation (owner facts, argument order, one identity).
    public static Node MakeNode(int value) => new Node(value);
    public static Node MakeDefault() => new Node();
    public static Sealed MakeSealed() => new Sealed();
    public static Box<int> MakeBox(int value) => new Box<int>(value);
    public static Outer.Inner MakeNested(int value) => new Outer.Inner(value);
    public static Pair MakeMixed(string text, int number) => new Pair(text, number);
    public static int ReadValue(Node node) => node.Value;
    public static int ReadAfterAllocation(int value) => new Node(value).Value;
    public static void WriteValue(Node node, int value) { node.Value = value; }
    public static int WriteAndRead(int value)
    {
        Node node = new Node(0);
        node.Value = value;
        return node.Value;
    }
    public static int UnsupportedStatic() => Node.Zero;
    public static int UnsupportedStructField(Point point) => point.X;
    public static int UnsupportedStruct() => new Point(1, 2).X;
}
#endif
