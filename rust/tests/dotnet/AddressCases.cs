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
    public uint ReadUnsigned(uint[] values, int index) { Effects = 141; Effects = 142; Effects = 143; Effects = 144; return 1001; }
    public uint ReadSignedBits(int[] values, int index) { Effects = 151; Effects = 152; Effects = 153; Effects = 154; return 1002; }
    public uint ReadSnapshot(uint[] values, int index, uint replacement) { Effects = 161; Effects = 162; Effects = 163; Effects = 164; return 1003; }
    public uint ReadAfterWrite(uint[] values, int index, uint replacement) { Effects = 171; Effects = 172; Effects = 173; Effects = 174; return 1004; }
    public uint ReadAfterRetarget(uint[] replacement, int index) { Effects = 181; Effects = 182; Effects = 183; Effects = 184; return 1005; }
    public int Bump() => ++Effects;
}
