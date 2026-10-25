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

namespace ConversionCases;

public class Cells
{
    public static int FromI8(long value) => unchecked((int)value);
    public static int FromU8(ulong value) => unchecked((int)value);
    public static int FromNative(nint value) => unchecked((int)value);
    public static int FromUNative(nuint value) => unchecked((int)value);
    public static int Snapshot(nint value, nint replacement)
    {
        nint old = value;
        value = replacement;
        return unchecked((int)old);
    }
    public static long ExtendI4(int value) => value;
    public static long ExtendU4Bits(uint value) => unchecked((long)(int)value);
    public static long ExtendNative(nint value) => value;
    public static long ExtendUNativeBits(nuint value) => unchecked((long)(nint)value);
    public static long ExtendSnapshot(int value, int replacement)
    {
        int old = value;
        value = replacement;
        return old;
    }
    public long Wide;
    public int ReadThenWrite(long replacement)
    {
        long old = Wide;
        Wide = replacement;
        return unchecked((int)old);
    }
}
