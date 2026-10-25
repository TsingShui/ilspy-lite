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

namespace ArrayCases;

public class Cells
{
    public static int ReadSigned(int[] values, int index) => values[index];
    public static uint ReadUnsigned(uint[] values, int index) => values[index];
    public static void WriteSigned(int[] values, int index, int value) => values[index] = value;
    public static void WriteUnsigned(uint[] values, int index, uint value) => values[index] = value;
    public static uint[] Null() => null;
    public uint[] Values;
    public uint OldThenWrite(int index, uint value)
    {
        uint[] snapshot = Values;
        uint old = snapshot[index];
        snapshot[index] = value;
        return old;
    }
    public static uint[] Choose(bool first, uint[] left, uint[] right) => first ? left : right;
    public int[] Numbers;
    public int Effects;
    private int Bump() => ++Effects;
    public void EffectfulStore(int index) => Numbers[index] = Bump();
    public static void Copy(uint[] target, uint[] source, int index) => target[index] = source[index];
    public static uint ReadChoice(bool first, uint[] left, uint[] right, int index) => (first ? left : right)[index];
    public static int LengthSigned(int[] values) => values.Length;
    public static int LengthUnsigned(uint[] values) => values.Length;
    public static long WideLength(uint[] values) => values.LongLength;
    public int LengthAfterEffect()
    {
        Bump();
        int length = Values.Length;
        Bump();
        return length;
    }
    public int LengthSnapshot(uint[] replacement)
    {
        uint[] old = Values;
        Values = replacement;
        return old.Length;
    }
    public static int CountLoop(uint[] values)
    {
        int count = 0;
        for (uint i = 0; i < values.Length; i++)
            count++;
        return count;
    }
}
