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

namespace GenericValueCases;

public struct Pair<T>
{
    public T First;
    public T Second;
}

public struct Box<T>
    where T : struct
{
    public T Value;
}

public struct Wrapper<T>
{
    public Pair<T> Inner;
}

public static class Ops
{
    // Whole-value plumbing: no member is read, so the only claim is that the
    // constructed value is copied exactly as the CLI copies it.
    public static Pair<int> Identity(Pair<int> pair) => pair;
    public static Pair<int> Pass(Pair<int> pair)
    {
        Pair<int> copy = pair;
        return copy;
    }
    public static Box<byte> Boxed(Box<byte> box) => box;
    public static Wrapper<string> Wrapped(Wrapper<string> wrapper) => wrapper;
    public static void Ignore(Pair<long> pair) { _ = pair; }

    // Field reads on a constructed value need the instantiated field type.
    public static int Sum(Pair<int> pair) => pair.First + pair.Second;
    public static T Peek<T>(Box<T> box) where T : struct => box.Value;
}
