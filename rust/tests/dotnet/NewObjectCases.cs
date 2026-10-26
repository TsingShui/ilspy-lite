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

namespace NewObjectCases;

public class Node
{
    public int Value;
    public object Tag;
    public Node Next;
    public Node() { }
    public Node(int value) { Value = value; }
    public Node(int value, object tag) { Value = value; Tag = tag; Next = this; }
    public int Read() => Value;
}

public class Chain
{
    public Node Head;
    public static int Made;
    public Chain() { Head = new Node(1); Made++; }
    public Chain(int value) : this() { Head.Value = value; }
}

public struct Holder
{
    public int Number;
    public Holder(int number) { Number = number; }
}

public abstract class Base
{
    protected Base() { }
}

public static class Ops
{
    public static Node Empty() => new Node();
    public static int Value(int value) => new Node(value).Read();
    public static object Tag(int value, object tag) => new Node(value, tag).Tag;
    public static Node One(int value) => new Node(value);
    public static int Two(int first, int second) => new Node(first).Value + new Node(second).Value;
    public static Node Shared(Node existing) => new Node(3, existing);
    public static int ChainValue(int value) => new Chain(value).Head.Value;
    public static Node A() => new Node(1);
    public static Node B() => new Node(1);

    public static object UnsupportedExternal(object value) => new string('x', 1);
    public static Holder UnsupportedStruct(int value) => new Holder(value);
    public static T UnsupportedGenericNew<T>() where T : new() => new T();
    public static string UnsupportedGeneric<T>(T value) => value?.ToString();
}
