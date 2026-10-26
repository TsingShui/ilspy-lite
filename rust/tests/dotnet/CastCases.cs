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

namespace CastCases;

public class Base
{
    public int Number;
}

public sealed class Derived : Base
{
    public int Extra;
}

public sealed class Other
{
    public int Value;
}

public static class Ops
{
    public static Derived Downcast(object value) => (Derived)value;
    public static Base Upcast(Derived value) => (Base)value;
    public static Derived As(object value) => value as Derived;
    public static bool IsDerived(object value) => value is Derived;
    public static int Guard(object value) => value is Derived ? 1 : 0;
    public static Derived Same(Derived value) => (Derived)value;
    public static Other Switch(object value) => value as Other;

    public static string UnsupportedString(object value) => value as string;
    public static Base Unrelated(Other value) => (Base)(object)value;
    public static int UnsupportedInterface(IComparable value) => value.CompareTo(null);
}
