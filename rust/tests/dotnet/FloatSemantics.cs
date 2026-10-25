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

// Test-only nominal R4 carrier for the local .NET 10 runtime. This is not the
// implementation-independent definition of CLI F or a production runtime library.
public static class CilFloat
{
    public static float __cil_ldc_r4_bits(uint bits) => System.BitConverter.Int32BitsToSingle(unchecked((int)bits));
    public static float __cil_div_F(float numerator, float denominator) => numerator / denominator;
    public static float __cil_conv_r4(int value) => unchecked((float)value);
    public static float __cil_conv_r4(long value) => unchecked((float)value);
    public static float __cil_conv_r4(nint value) => unchecked((float)value);
    public static float __cil_arg_r4(float value) => value;
    public static float __cil_set_arg_r4(float value) => value;
    public static float __cil_load_r4(float value) => value;
    public static float __cil_store_r4(float value) => value;
    public static float __cil_call_arg_r4(float value) => value;
    public static float __cil_return_r4(float value) => value;
}
