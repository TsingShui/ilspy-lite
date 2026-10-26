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
namespace ReferenceCalls;
public interface IBox<T> { T Take(T value); }
public class Box<T> : IBox<T>
{
    public T Value;
    public int Calls;
    public virtual T Get() { Calls++; if (Calls == 8) throw new System.InvalidOperationException("read"); return Value; }
    public T Take(T value) { Calls++; T old = Value; Value = value; return old; }
    public T this[int index] { get { Calls += index; if (index < 0) throw new System.ArgumentOutOfRangeException(nameof(index)); return Value; } }
    public static int Static(int value) => value + 19;
    public override string ToString() { Calls++; return "box"; }
}
public class DerivedBox<T> : Box<T>
{
    public override T Get() { Calls += 10; return Value; }
}
public class HiddenBox<T> : Box<T>
{
    public new T Get() { Calls += 1000; return default; }
}
public class ExplicitBox<T> : IBox<T>
{
    public T Value;
    public int Calls;
    T IBox<T>.Take(T value) { Calls += 3; T old = Value; Value = value; return old; }
}
public class Log
{
    public int Events;
    public int Argument(int divisor) { Events++; return 100 / divisor; }
}
public class Gvm { public virtual T Echo<T>(T value) => value; }
#else
using ReferenceCalls;
namespace ReferenceCallCases;
public static class Ops
{
    public static int IntGet(Box<int> receiver) => receiver.Get();
    public static string StringGet(Box<string> receiver) => receiver.Get();
    public static int IntTake(Box<int> receiver, int value) => receiver.Take(value);
    public static string StringTake(Box<string> receiver, string value) => receiver.Take(value);
    public static int Interface(IBox<int> receiver, int value) => receiver.Take(value);
    public static int Indexed(Box<int> receiver, int index) => receiver[index];
    public static int Static(int value) => Box<int>.Static(value);
    public static int ArgumentOrder(Log log, Box<int> receiver, int divisor) => receiver.Take(log.Argument(divisor));
    public static int[] ArrayClone(int[] value) => (int[])value.Clone();
    public static string CoreString(object value) => value.ToString();
    public static int CoreLength(string value) => value.Length;
    public static Box<int> Cast(object value) => (Box<int>)value;
    public static IBox<int> As(object value) => value as IBox<int>;
    public static int UnsupportedGvm(Gvm value) => value.Echo<int>(7);
    public static int UnsupportedByRef(ref int value) => value;
}
#endif
