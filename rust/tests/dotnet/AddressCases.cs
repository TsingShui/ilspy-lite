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

namespace AddressCases;

public class Cells
{
    public int Effects;
    public uint[] Values;
    public static ref uint AnchorUnsigned(uint[] values, int index) => ref values[index];
    public static ref int AnchorSigned(int[] values, int index) => ref values[index];
    // Producer replaces these reserved bodies with authored IL before tests run.
    public void Take(uint[] values, int index) { Effects = 101; Effects = 102; Effects = 103; Effects = 104; }
    public void TakeSigned(int[] values, int index) { Effects = 111; Effects = 112; Effects = 113; Effects = 114; }
    public void Duplicate(uint[] values, int index) { Effects = 121; Effects = 122; Effects = 123; Effects = 124; }
    public void Retarget(uint[] replacement, int index) { Effects = 131; Effects = 132; Effects = 133; Effects = 134; Effects = 135; }
    public int Bump() => ++Effects;
}
