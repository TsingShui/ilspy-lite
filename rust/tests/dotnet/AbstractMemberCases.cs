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

namespace AbstractMemberCases;

public abstract class Base<T>
{
    public abstract int Compute(int value);
    public int Fixed() => 7;
}
public class Derived<T> : Base<T>
{
    public int Calls;
    public override int Compute(int value) { Calls++; return value + 100; }
}
public class Other<T> : Base<T>
{
    public override int Compute(int value) => value - 100;
}
public interface IValue<T> { int Compute(int value); }
public class Buildable<T> { public Buildable() { } }
public static class Ops
{
    public static int Virtual(Base<int> receiver, int value) => receiver.Compute(value);
    public static int Fixed(Base<int> receiver) => receiver.Fixed();
    public static int Twice(Base<int> receiver, int value) => receiver.Compute(value) + receiver.Compute(value);
    public static int Interface(IValue<int> receiver, int value) => receiver.Compute(value);
    public static Buildable<int> Allocate() => new Buildable<int>();
}
