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

namespace EqualityCases;

public sealed class Item
{
    public int Value;
}

public static class Ops
{
    public static bool Same(object left, object right) => ReferenceEquals(left, right);
    public static bool IsNull(object value) => value == null;
    public static bool NotNull(object value) => value != null;
    public static bool SameTyped(Item left, Item right) => left == right;
    public static bool BothNull(object left, object right) => left == null && right == null;
    public static int NullGuard(object value) => value == null ? 0 : 1;
    public static object Choose(object value, object fallback) => value == null ? fallback : value;
    public static bool StringNull(string value) => value == null;

    public static bool UnsupportedCast(object value) => (Item)value != null;
    public static bool UnsupportedTypeTest(object value) => value is Item;
    public static object UnsupportedExternal(object value) => value.Equals(null);
}
