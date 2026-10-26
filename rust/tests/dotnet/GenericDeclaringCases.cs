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

namespace GenericDeclaringCases;

public class Holder<T>
{
    public T Value;
    public int Count;

    public Holder(int count)
    {
        Count = count;
    }

    public virtual int Score() => Count;

    public Holder<T> Self() => this;
}

public class Derived<T> : Holder<T>
{
    public Derived(int count)
        : base(count)
    {
    }

    public override int Score() => Count + 1;
}

public struct Slot<T>
{
    public T Content;
}

public enum Kind : byte { A, B }

public static class Ops
{
    public static int CallScore(Holder<int> holder) => holder.Score();
    public static Holder<int> MakeHolder() => new Holder<int>(3);
    public static Derived<int> MakeDerived() => new Derived<int>(4);
    public static Holder<int> Identity(Holder<int> holder) => holder;
    public static int Field(Holder<int> holder) => holder.Count;
    public static Holder<int> SelfCall(Holder<int> holder) => holder.Self();
    public static int UnsupportedContent(Holder<int> holder) => holder.Value;
    public static Slot<int> UnsupportedStruct(Slot<int> slot) => slot;
    public static object BoxInt(int value) => value;
    public static object BoxEnum(Kind kind) => kind;
    public static object BoxStruct(Slot<int> slot) => slot;
    public static int UnboxInt(object value) => (int)value;
    public static object UnsupportedBoxGeneric<T>(T value) => value;
}
