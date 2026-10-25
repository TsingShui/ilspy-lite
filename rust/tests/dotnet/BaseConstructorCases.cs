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
namespace BaseConstructorLibrary;
#else
namespace BaseConstructorCases;
#endif

public class Base
{
    public int First;
    public int Second;
    public int Calls;
    public Base() { }
    public Base(int first, int second)
    {
        Calls = unchecked(Calls + 1);
        First = first;
        if (first == -17) throw new System.ArgumentOutOfRangeException("first", "stop-first");
        Second = second;
        if (second == -19) throw new System.ArgumentOutOfRangeException("second", "stop-second");
    }
}

#if !DEPENDENCY
public static class Probe
{
    public static int Trace;
    public static int Calls;
    public static int Mark(int value)
    {
        Trace = unchecked(Trace * 31 + value);
        Calls = unchecked(Calls + 1);
        return value;
    }
}

public class Local : Base
{
    public readonly int Before = Probe.Mark(1);
    public readonly int After;
    public Local(int first, int second) : base(Probe.Mark(first), Probe.Mark(second))
    {
        After = Probe.Mark(7);
    }
}

public class External : BaseConstructorLibrary.Base
{
    public readonly int Before = Probe.Mark(1);
    public readonly int After;
    public External(int first, int second) : base(Probe.Mark(first), Probe.Mark(second))
    {
        After = Probe.Mark(7);
    }
}
#endif
