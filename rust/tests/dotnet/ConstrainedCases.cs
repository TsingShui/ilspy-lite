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


namespace ConstrainedCases
{
    public interface IAccumulator { int Step(int delta); }
    public static class Ledger
    {
        public static int Trace;
        public static int Argument(int delta) { Trace = unchecked(Trace * 10 + 1); return delta; }
        public static int Replace<T>(ref T receiver, T replacement, int delta)
        { receiver = replacement; Trace = unchecked(Trace * 10 + 2); return delta; }
    }
    public struct Counter : IAccumulator
    {
        public int Value;
        public string Text;
        public int Step(int delta) { Value = unchecked(Value + delta); Ledger.Trace += 100; return 100 / delta; }
        public override string ToString() { Value++; Ledger.Trace += 1000; return Text; }
    }
    public struct Plain { public int Value; public string Text; }
    public class CounterClass : IAccumulator
    {
        public int Value;
        public string Text;
        public virtual int Step(int delta) { Value = unchecked(Value + delta); Ledger.Trace += 200; return 100 / delta; }
        public override string ToString() { Value++; Ledger.Trace += 2000; return Text; }
    }
    public sealed class DerivedCounter : CounterClass
    {
        public override int Step(int delta) { Value = unchecked(Value - delta); Ledger.Trace += 300; return 200 / delta; }
    }
    public static class Host<T>
    {
        public static string Text(ref T receiver) => receiver.ToString();
    }
    public static class Ops
    {
        public static string Text<T>(ref T receiver) => receiver.ToString();
        public static string MaybeText<T>(T receiver) => receiver?.ToString();
        public static int Step<T>(ref T receiver, int delta) where T : IAccumulator => receiver.Step(delta);
        public static int Copy<T>(T receiver, int delta) where T : IAccumulator => receiver.Step(delta);
        public static int Order<T>(ref T receiver, int delta) where T : IAccumulator => receiver.Step(Ledger.Argument(delta));
        public static int Slot<T>(ref T receiver, T replacement, int delta) where T : IAccumulator => receiver.Step(Ledger.Replace(ref receiver, replacement, delta));
        public static string Concrete(ref Plain receiver) => receiver.ToString();
        public static int Conditional(bool first, ref Counter left, ref Counter right, int delta) => (first ? ref left : ref right).Step(delta);
        public static ref uint ArrayAnchor(uint[] values, int index) => ref values[index];
        public static uint ArrayCarry(uint[] values, int index, bool first)
        { Ledger.Trace = 1; Ledger.Trace = 2; Ledger.Trace = 3; Ledger.Trace = 4; return values[index]; }
        public static int Local(bool first, ref Counter left, ref Counter right, int delta)
        { ref Counter selected = ref left; if (!first) selected = ref right; return selected.Step(delta); }
    }
}
