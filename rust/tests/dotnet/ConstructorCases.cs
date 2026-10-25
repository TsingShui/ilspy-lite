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

namespace ConstructorCases;

public class Base
{
    public readonly int BaseValue;
    public Base(int value) { BaseValue = value; }
}

public class Cells : Base
{
    public readonly uint[] Values;
    public readonly int Initial;
    public int Effects;
    public static readonly int StaticValue = 3;
    public Cells(int length, int value) : base(value)
    {
        Effects = 1;
        Values = new uint[length];
        Initial = value;
        Effects = 2;
    }
    public Cells() : this(3, 7) { Effects = 4; }
    public Cells(Cells other, int value) : base(value) { Initial = value; }
    public int Read() => Initial;
    public int Reserved(int value) { Effects = 101; Effects = 102; Effects = 103; return value; }
}
