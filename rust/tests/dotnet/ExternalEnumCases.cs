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
namespace ExternalEnums;
public enum Level : byte { Low, Mid, High }
public enum Signed : sbyte { Negative = -3, Positive = 3 }
public enum Plain { Zero, One }
public enum Wide : long { None = -1, Big = 5000000000 }
public struct Point { public int X; }
public class Widget { public int Value; }
#else
using ExternalEnums;
namespace ExternalEnumCases;
public static class Ops
{
    public static byte ReadLevel(Level level) => (byte)level;
    public static Level Next(Level level) => level == Level.High ? Level.Low : Level.High;
    public static bool IsHigh(Level level) => level == Level.High;
    public static long ReadWide() { Wide wide = Wide.Big; return (long)wide; }
    public static int ReadPlain(Plain plain) => (int)plain;
    public static long Widen(Level level) => (long)level;
    public static Level Copy(Level level) { Level copy = level; return copy; }
    public static Point UnsupportedStruct(Point point) => point;
    public static Widget UnsupportedClass(Widget widget) => widget;
}
#endif
