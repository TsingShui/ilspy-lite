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
namespace ExternalValues;
public struct Measure
{
    public int Units;
    public byte Scale;
}
public struct Token
{
    public int Id;
    public byte Tag;
}
public struct Nested
{
    public Measure Inner;
}
public enum Level : byte { Low, High }
// A byref-like definition may never be copied as an ordinary value.
public ref struct Cursor
{
    public int Position;
}
#else
using ExternalValues;
namespace ExternalValueCases;
public static class Ops
{
    public static Measure Identity(Measure measure) => measure;
    public static Measure Pass(Measure measure)
    {
        Measure copy = measure;
        return copy;
    }
    public static Token Keep(Token token) => token;
    public static Nested Wrap(Nested nested) => nested;
    public static Level Level(Level level) => level;
    public static void Ignore(Measure measure) { _ = measure; }
    public static Measure UnsupportedRefStruct(Cursor cursor) => default;
    public static int UnsupportedField(Measure measure) => measure.Units;
}
#endif
