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

namespace FloatCases;

public class Cells
{
    public int Count;
    public float Number;
    public static float FromI4(int value) => value;
    public static float FromU4Bits(uint value) => unchecked((int)value);
    public static float FromI8(long value) => value;
    public static float FromU8Bits(ulong value) => unchecked((long)value);
    public static float FromNative(nint value) => value;
    public static float FromUNativeBits(nuint value) => unchecked((nint)value);
    public static float Identity(float value) => value;
    public static float Choose(bool first, float left, float right) => first ? left : right;
    public float Replace(float replacement)
    {
        float old = Number;
        Number = replacement;
        return old;
    }
    public float ConvertedSnapshot(int replacement)
    {
        float old = Count;
        Count = replacement;
        return old;
    }
    public static float Unit() => 1.0f;
    public static float NegativeZero() => -0.0f;
    public static float Largest() => float.MaxValue;
    public static float Infinite() => float.PositiveInfinity;
    public static float NotANumber() => float.NaN;
    public static float Smallest() => float.Epsilon;
    public static float Divide(float numerator, float denominator) => numerator / denominator;
    public static float Scale(int value) => value / (float)int.MaxValue;
    public float DivideSnapshot(float denominator, float replacement)
    {
        float result = Number / denominator;
        Number = replacement;
        return result;
    }
    public static float Absolute(float value) => System.Math.Abs(value);
    public float AbsoluteSnapshot(float replacement)
    {
        float old = Number;
        Number = replacement;
        return System.Math.Abs(old);
    }
    public static float RecoveredUnsigned(uint value) => value;
    public static float UnsupportedUnsignedNative(nuint value) => value;
    public static double RecoveredDouble(double value) => value;
    public static double RecoveredDoubleUnbox(object value) => (double)value;
}
