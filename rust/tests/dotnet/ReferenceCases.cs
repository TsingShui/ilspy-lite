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

namespace ReferenceCases;

public class Cell
{
    public int Value;
    public Cell Other;
    public int Count { get; set; }
    public int Read() => Value;
    public int Credit(int delta)
    {
        int old = Value;
        Value += delta;
        return old;
    }
    public int Nested() => Other.Value;
    public int CountTwice() => Count + Count;
    public virtual int Constant() => 41;
    public static int StoreAfterRead(Cell target, Cell source)
    {
        int value = source.Value;
        target.Value = value;
        return value;
    }
    public static Cell Choose(bool first, Cell left, Cell right) => first ? left : right;
    public static Cell Null() => null;
    public static Cell ClearOther(Cell target) { target.Other = null; return target; }
}
public class Child : Cell
{
    public override int Constant() => 99;
    public int BaseConstant() => base.Constant();
}
public class ExternalBase : System.Exception { public int Number; public int Read() => Number; }
public struct ValueCell { public int Value; public int Read() => Value; }
