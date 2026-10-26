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
using System;

namespace ExternalProperties;

public class Cell
{
    public int Value;
    public int Writes;
    public int Reads;
    public virtual int Number
    {
        get { Reads++; if (Value == 7) throw new InvalidOperationException("getter"); return Value + 1; }
        set { Writes++; if (value == -7) throw new ArgumentException("setter"); Value = value; }
    }
    public bool Flag { get; set; }
    public int get_Orphan() => 3;
    public int this[int index] => Value + index;
}
public class Derived : Cell
{
    public override int Number
    {
        get { Reads += 10; return Value + 100; }
        set { Writes += 10; Value = value * 2; }
    }
}
public class Hidden : Cell
{
    public new int Number
    {
        get { Reads += 50; return -100; }
        set { Writes += 50; Value = -value; }
    }
}
public class Generic<T> { public T Item { get; set; } }
public interface ICell { int Number { get; } }
#else
using ExternalProperties;
namespace ExternalPropertyCases;

public static class Ops
{
    public static int Get(Cell cell) => cell.Number;
    public static void Set(Cell cell, int value) { cell.Number = value; }
    public static bool GetFlag(Cell cell) => cell.Flag;
    public static void SetFlag(Cell cell, bool value) { cell.Flag = value; }
    public static int ReadTwice(Cell cell) => cell.Number + cell.Number;
    public static int SetAndGet(Cell cell, int value) { cell.Number = value; return cell.Number; }
    public static void SetComputed(Cell cell, int value) { cell.Number = 100 / value; }
    public static int Orphan(Cell cell) => cell.get_Orphan();
    public static int Indexed(Cell cell, int value) => cell[value];
    public static int Generic(Generic<int> cell) => cell.Item;
    public static int Interface(ICell cell) => cell.Number;
}
#endif
