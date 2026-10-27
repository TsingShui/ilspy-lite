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


#if LIBRARY
namespace ValueCallLibrary
{
    public static class Probe
    {
        public static int Divisor = 1;
        public static int Entries;
        public static int Trace;
        public static int Argument(int value) { Trace = unchecked(Trace * 10 + 1); return value; }
    }
    public struct Cell<T>
    {
        public T Value;
        public int Count;
        public Cell(T value) { Value = value; Probe.Entries++; Count = 100 / Probe.Divisor; }
        public T Read() { Count++; return Value; }
        public void Write(T value) { Value = value; Count++; }
        public static T Echo(T value) => value;
        public int ReadThrow(int divisor) { Count++; return 100 / divisor; }
        public U Exchange<U>(ref U input, int divisor)
        {
            U result = input;
            Count++;
            Probe.Trace = unchecked(Probe.Trace * 10 + 7);
            input = default;
            if (divisor == 0) throw new System.DivideByZeroException();
            return result;
        }
    }
}
#else
namespace ValueCallCases
{
    public static class Probe
    {
        public static int Divisor = 1;
        public static int Entries;
        public static int Trace;
        public static int Argument(int value) { Trace = Trace * 10 + 1; return value; }
        public static object ArgumentTag(object value) { Trace = Trace * 10 + 2; if (value == null) throw null; return value; }
    }
    public struct Cell<T>
    {
        public T Value;
        public int Count;
        public Cell(T value) { Value = value; Probe.Entries++; Count = 100 / Probe.Divisor; }
        public T Read() { Count++; return Value; }
        public void Write(T value) { Value = value; Count++; }
        public static T Echo(T value) => value;
        public int ReadThrow(int divisor) { Count++; return 100 / divisor; }
    }
    public struct Plain
    {
        public readonly int Value;
        public object Tag;
        public int Count;
        public Plain(int value) : this(value, null) { Probe.Trace = unchecked(Probe.Trace * 10 + 4); }
        public Plain(ref int value, ref object tag) {
            Value = value; value++;
            Probe.Entries++;
            Probe.Trace = unchecked(Probe.Trace * 10 + 5);
            Count = 100 / Probe.Divisor;
            Tag = tag; tag = null;
        }
        public Plain(int value, object tag) {
            Value = value;
            Probe.Entries++;
            Probe.Trace = unchecked(Probe.Trace * 10 + 3);
            Count = 100 / Probe.Divisor;
            Tag = tag;
        }
    }
    public static class Ops
    {
        public static Plain NewPlainRef(ref int value, ref object tag) => new Plain(ref value, ref tag);
        public static void NewPlainAliased(ref Plain target) { target = new Plain(ref target.Count, ref target.Tag); }
        public static Plain NewPlainDelegated(int value) => new Plain(value);
        public static void ReinitializePlain(ref Plain target, int value, object tag) { target = new Plain(value, tag); }
        public static Plain NewPlain(int value, object tag) => new Plain(value, tag);
        public static void NewPlainInto(ref Plain target, int value, object tag) { target = new Plain(value, tag); }
        public static Plain NewPlainOrder(int value, object tag) => new Plain(Probe.Argument(value), Probe.ArgumentTag(tag));
        public static object Box(Cell<string> value) => value;
        public static object BoxGeneric<T>(T value) => value;
        public static bool SameBox<T>(T value) { object box = value; return object.ReferenceEquals(box, box); }
        public static bool DifferentBoxes<T>(T value) => object.ReferenceEquals(value, value);
        public static T Unbox<T>(object value) => (T)value;
        public static Cell<string> UnboxCell(object value) => (Cell<string>)value;
        public static object CopyBox(ref Cell<int> value) { object result = value; value.Count = 99; return result; }
        public static Cell<T> New<T>(T value) => new Cell<T>(value);
        public static void NewInto<T>(ref Cell<T> target, T value) { target = new Cell<T>(value); }
        public static Cell<int> NewOrder(int value) => new Cell<int>(Probe.Argument(value));
        public static ValueCallLibrary.Cell<T> ForeignNew<T>(T value) => new ValueCallLibrary.Cell<T>(value);
        public static int Read(ref Cell<int> value) => value.Read();
        public static int Throw(ref Cell<int> value, int divisor) => value.ReadThrow(divisor);
        public static string CopyRead(Cell<string> value) => value.Read();
        public static int StringCount(ref Cell<string> value) => value.Count;
        public static T Generic<T>(ref Cell<T> value) => value.Read();
        public static int Alias(ref Cell<int> value) => value.Read() + value.Read() + value.Count;
        public static void Write(ref Cell<object> value, object next) => value.Write(next);
        public static string Static(string value) => Cell<string>.Echo(value);
        public static int Foreign(ref ValueCallLibrary.Cell<int> value) => value.Read();
        public static T ForeignGeneric<T>(ref ValueCallLibrary.Cell<T> value) => value.Read();
        public static void ForeignWrite(ref ValueCallLibrary.Cell<object> value, object next) => value.Write(next);
        public static int ForeignExchange(ref ValueCallLibrary.Cell<int> value, ref int input, int divisor) => value.Exchange<int>(ref input, divisor);
        public static string ForeignExchangeString(ref ValueCallLibrary.Cell<int> value, ref string input, int divisor) => value.Exchange<string>(ref input, divisor);
        public static int ForeignExchangeCopy(ValueCallLibrary.Cell<int> value, ref int input, int divisor)
        {
            int result = value.Exchange<int>(ref input, divisor);
            return result + value.Count;
        }
        public static int ForeignExchangeAliased(ref ValueCallLibrary.Cell<int> value, int divisor) => value.Exchange<int>(ref value.Count, divisor);
        public static int ForeignExchangeOrder(ref ValueCallLibrary.Cell<int> value, ref int input, int divisor) => value.Exchange<int>(ref input, ValueCallLibrary.Probe.Argument(divisor));
        public static T ForeignExchangeGeneric<T>(ref ValueCallLibrary.Cell<T> value, ref T input, int divisor) => value.Exchange<T>(ref input, divisor);
        public static T ForeignExchangeCross<T>(ref ValueCallLibrary.Cell<int> value, ref T input, int divisor) => value.Exchange<T>(ref input, divisor);
    }
}
#endif
