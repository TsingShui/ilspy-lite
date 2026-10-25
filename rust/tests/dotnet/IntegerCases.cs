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

namespace IntegerCases;

public static class Cases
{
    public static int state;
    public static int Add(int a, int b) => unchecked(a + b);
    public static uint AddCheckedUnsigned(uint a, uint b) => checked(a + b);
    public static uint SubCheckedUnsigned(uint a, uint b) => checked(a - b);
    public static uint MulCheckedUnsigned(uint a, uint b) => checked(a * b);
    public static long Add64(long a, long b) => unchecked(a + b);
    public static long MulChecked64(long a, long b) => checked(a * b);
    public static ulong AddCheckedU64(ulong a, ulong b) => checked(a + b);
    public static int Div(int a, int b) => a / b;
    public static uint DivUnsigned(uint a, uint b) => a / b;
    public static int Rem(int a, int b) => a % b;
    public static ulong RemUnsigned64(ulong a, ulong b) => a % b;
    public static int Neg(int a) => unchecked(-a);
    public static long Not64(long a) => ~a;
    public static int XorOr(int a, int b) => (a ^ b) | a;
    public static long ShiftLeft64(long a, int b) => a << b;
    public static int ShiftRight(int a, int b) => a >> b;
    public static ulong ShiftRightU64(ulong a, int b) => a >> b;
    public static bool GreaterUnsigned(uint a, uint b) => a > b;
    public static bool Less64(long a, long b) => a < b;
    public static bool Equal(int a, int b) => a == b;
    public static int Join(bool a, int b, int c) => (a ? b : c) + (a ? c : b);
    public static int Next() => ++state;
    public static int PopCall() { state = 0; Next(); return Next(); }
    public static int DupLocal(int a) { int b; return (b = a) + b; }
    public static int External(int a) => System.Math.Abs(a);
}
