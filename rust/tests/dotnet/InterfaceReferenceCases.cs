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

namespace InterfaceReferenceCases;

public interface IValue { int Compute(int value); }
public interface IChild : IValue { }
public interface IGeneric<T> { int Compute(T value); }
public class Counter : IChild
{
    public int Calls;
    public int Compute(int value) { Calls++; return value + 21; }
    public int Record(int value) { Calls += 10; return 100 / value; }
}
public class DerivedCounter : Counter { }
public class Explicit : IValue
{
    public int Calls;
    int IValue.Compute(int value) { Calls++; return value - 21; }
    public int Compute(int value) => -999;
}
public struct Boxed : IValue
{
    public int Calls;
    public int Compute(int value) { Calls++; return value + 3; }
}
public class Holder { public IValue Field; }
public class Unrelated { }

public static class Ops
{
    public static IValue Pass(IValue value) => value;
    public static IValue Cast(object value) => (IValue)value;
    public static IValue As(object value) => value as IValue;
    public static bool Is(object value) => value is IValue;
    public static int Invoke(IValue receiver, int value) => receiver.Compute(value);
    public static int Twice(IValue receiver, int value) => receiver.Compute(value) + receiver.Compute(value);
    public static IValue ToBase(IChild value) => value;
    public static IValue FromClass(Counter value) => value;
    public static IValue FromDerived(DerivedCounter value) => value;
    public static IValue Read(Holder holder) => holder.Field;
    public static IValue Exchange(Holder holder, IValue value) { IValue old = holder.Field; holder.Field = value; return old; }
    public static int ArgumentOrder(Counter log, IValue receiver, int value) => receiver.Compute(log.Record(value));
    public static int UnsupportedGeneric(IGeneric<int> receiver, int value) => receiver.Compute(value);
    public static Counter Allocate() => new Counter();
}
