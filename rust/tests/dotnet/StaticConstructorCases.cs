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
using System.Text;

namespace StaticConstructorCases;

// Field initializers merged into one static constructor: constants first, then a
// string literal.
public static class Simple
{
    public static int Seed;
    public static string Label;

    static Simple()
    {
        Seed = 17;
        Label = "seed";
    }
}

// Not idempotent: the accumulated total is read before it is written, so running
// the body twice is observable.
public static class Accumulator
{
    public static int Total;

    static Accumulator()
    {
        Total = Total + 5;
        Total = Total + 7;
    }
}

// Two-phase initialization across types: First's static constructor reads the
// static field of Second, which initializes Second first.
public static class First
{
    public static readonly int Value;

    static First()
    {
        Value = Second.Value + 1;
    }
}

public static class Second
{
    public static int Value;

    static Second()
    {
        Value = 41;
    }
}

// A struct's static constructor initializes only static state.
public struct Sized
{
    public static int Width;

    public int Value;

    static Sized()
    {
        Width = 3;
    }
}

// A throwing static constructor: the store before the throw is observable and the
// failure surfaces as the platform wraps it.
public static class Failing
{
    public static int Attempts;

    static Failing()
    {
        Attempts = 1;
        throw new ArgumentOutOfRangeException("value", "static init failed");
    }
}

// Delegate caching: the method group conversion allocates one delegate and stores
// it in a static readonly field.
public static class Caches
{
    public static Func<int, int> Twice = TwiceImpl;

    public static int TwiceImpl(int value) => value * 2;
}

// A static field of a generic type; the storing constructor is the type's own.
public static class Registry<T>
{
    public static int Count;
    public static readonly int Stamp;

    static Registry()
    {
        Count = 1;
        Stamp = Count + 7;
    }
}

// Allocation of a type defined in another assembly, stored in a static readonly
// field.
public static class ExternalCache
{
    public static StringBuilder Holder = new StringBuilder();
}
