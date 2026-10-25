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

namespace PropertyCases;

public class Cell : BindingLibrary.Base
{
    public new bool Enabled => false;
    public bool OwnConstant => true;
    public bool ReadOwnConstant() => OwnConstant;
    public int Counter;
    public int Calls;
    public uint UnsignedCounter;
    public Cell Other;
    public bool Decrement() => unchecked(Counter-- == 0);
    public bool CheckedDecrement() => checked(Counter-- == 0);
    public bool OtherDecrement(Cell other) => unchecked(other.Counter-- == 0);
    public Cell Fetch() { Calls = unchecked(Calls + 1); return Other; }
    public bool EffectfulReceiver() => unchecked(Fetch().Counter-- == 0);
    public bool KeepOldAcrossEffect()
    {
        int old = Counter;
        Counter = unchecked(old - 1);
        Calls = unchecked(Calls + 1);
        return old == 0;
    }
    public bool DifferentLocation(Cell other)
    {
        int old = Counter;
        other.Counter = unchecked(old - 1);
        return old == 0;
    }
    public bool TwoReads()
    {
        int old = Counter;
        Counter = unchecked(old - 1);
        return Counter == old;
    }
    public bool UnsignedDecrement() => unchecked(UnsignedCounter-- == 0);
    public bool LocalDecrement(int value)
    {
        int current = value;
        bool wasZero = unchecked(current-- == 0);
        Calls = current;
        return wasZero;
    }
    public bool EffectBetweenReadAndWrite()
    {
        int old = Counter;
        Calls = unchecked(Calls + 1);
        Counter = unchecked(old - 1);
        return old == 0;
    }
    public bool ReadBase() => base.Enabled;
    public bool Collision()
    {
        bool first = base.Enabled;
        return first & x__cil_memberref_call_FFFFFFFF(this);
    }
    public static bool x__cil_memberref_call_FFFFFFFF(Cell value) => false;
    public string Literal()
    {
        if (base.Enabled)
            return "__cil_memberref_call_FFFFFFFF(this)";
        return null;
    }
    public bool ReassignReceiver(Cell other)
    {
        if (other == null)
            return base.Enabled;
        return base.ConstantFlag;
    }
    public bool ReadVirtualReceiver(Cell other) => other.VirtualFlag;
    public bool ReadConstantReceiver(Cell other) => other.ConstantFlag;
}
