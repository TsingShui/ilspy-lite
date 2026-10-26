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
using System.Linq;

namespace VirtualCallCases;

public abstract class Shape
{
    public abstract int Area();
    public virtual int Sides() => 4;
    public virtual string Describe() => "shape" + Area();
    public int Fixed() => 7;
}

public sealed class Square : Shape
{
    public int Size;
    public override int Area() => Size * Size;
    public override string Describe() => "square";
}

public class Plain
{
    public int Value;
    public int Read() => Value;
}

public static class Ops
{
    public static int Area(Shape shape) => shape.Area();
    public static int Sides(Shape shape) => shape.Sides();
    public static string Describe(Shape shape) => shape.Describe();
    public static int Fixed(Shape shape) => shape.Fixed();
    public static int Both(Shape first, Shape second) => first.Area() + second.Area();
    public static int Nested(Shape shape) => shape.Describe().Length;
    public static int Double(Shape shape) => shape.Area() * 2;
    public static int Local(Shape shape)
    {
        int total = 0;
        for (int index = 0; index < 3; index++) total += shape.Area();
        return total;
    }

    public static int UnsupportedInterface(System.Collections.Generic.IEnumerable<int> values) => values.Count();
    public static int UnsupportedNew(int size) => new Square { Size = size }.Area();
    public static object UnsupportedBox(Shape shape) => shape;
    public static string UnsupportedConstrained(Value value) => value.ToString();
}

public struct Value
{
    public int Number;
    public override string ToString() => "value" + Number;
}

public static class Compute
{
    public static int Sum(Shape[] shapes) => shapes.Length;
}
