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

namespace GenericBindingCases;

public static class Tracker { public static int Count; }
public struct Payload { public int Number; public object Reference; }

public class Bag<T>
{
    static Bag() { Tracker.Count++; }
    public T Value;
    public T Get() => Value;
    public void Set(T value) { Value = value; }
    public T ForwardGet() => Get();
    public void ForwardSet(T value) { Set(value); }
    public Bag<T> Self() => this;
    public Bag<T> ForwardSelf() => Self();
    public static T Echo(T value) => value;
    public static T ForwardStatic(T value) => Echo(value);
}
public class PrefixBase<T> { public PrefixBase() { } }
public class Prefix<T, U> : PrefixBase<T> { public Prefix() : base() { } }

public class Proxy<T>
{
    public Bag<T> Target;
    public T Read() => Target.Value;
    public T Call() => Target.Get();
    public T Exchange(T value) { T old = Target.Value; Target.Value = value; return old; }
}
public class Pair<T, U>
{
    public T First;
    public U Second;
    public T GetFirst() => First;
    public U GetSecond() => Second;
}
public class Swapped<T, U>
{
    public Pair<U, T> Target;
    public U First() => Target.GetFirst();
    public T Second() => Target.GetSecond();
}
