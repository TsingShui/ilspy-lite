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

namespace ObjectCases;

public class Cell
{
    public object Value;
    public static object Global;
    public static string GlobalText;
    public static Cell GlobalCell;
    public static int[] GlobalArray;
    public object Read() => Value;
    public object Exchange(object next)
    {
        object before = Value;
        Value = next;
        return before;
    }
}

public static class Ops
{
    public static object Identity(object value) => value;
    public static object Null() => null;
    public static object StringAsObject(string value) => value;
    public static object ArrayAsObject(int[] value) => value;
    public static object UnsignedArrayAsObject(uint[] value) => value;
    public static object CellAsObject(Cell value) => value;
    public static object Choose(bool first, object left, object right) => first ? left : right;
    public static object Snapshot(object value)
    {
        object saved = value;
        value = null;
        return saved;
    }
    public static object AssignArgument(object value, object next)
    {
        value = next;
        return value;
    }
    public static object Forward(object value) => Identity(value);
    public static object ReadField(Cell cell) => cell.Value;
    public static object ExchangeField(Cell cell, object next)
    {
        object before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static object ReadAndClear(Cell cell)
    {
        object before = cell.Value;
        cell.Value = null;
        return before;
    }
    public static object StaticExchange(object next)
    {
        object before = Cell.Global;
        Cell.Global = next;
        return before;
    }
    public static object StaticStringExchange(string next)
    {
        string before = Cell.GlobalText;
        Cell.GlobalText = next;
        return before;
    }
    public static object StaticCellExchange(Cell next)
    {
        Cell before = Cell.GlobalCell;
        Cell.GlobalCell = next;
        return before;
    }
    public static object StaticArrayExchange(int[] next)
    {
        int[] before = Cell.GlobalArray;
        Cell.GlobalArray = next;
        return before;
    }
    public static object UnsupportedBox(int value) => value;
    public static int UnsupportedUnbox(object value) => (int)value;
    public static string UnsupportedCast(object value) => (string)value;
    public static string UnsupportedDispatch(object value) => value.ToString();
    public static object UnsupportedAllocation() => new object();
    public static object UnsupportedObjectArray(object[] value) => value;
}
