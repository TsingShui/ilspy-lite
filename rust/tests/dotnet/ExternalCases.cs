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
namespace ExternalLibrary;
public class Actor { }
public class Other { }
public class Base
{
    public bool Canceling;
    public int Reads;
    public bool ReadFlag() { Reads++; return Canceling; }
    public bool Constant() => true;
    public virtual int Virtual() => 41;
}
#else
namespace ExternalCases;
public class Cells : ExternalLibrary.Base
{
    public int Value;
    public int Unused(ExternalLibrary.Actor actor) => Value;
    public static ExternalLibrary.Actor Identity(ExternalLibrary.Actor actor) => actor;
    public static ExternalLibrary.Other Other(ExternalLibrary.Other value) => value;
    public bool DirectFlag(ExternalLibrary.Actor actor) => base.ReadFlag();
    public bool DirectConstant() => base.Constant();
    public int DirectVirtual() => base.Virtual();
    public override int Virtual() => 99;
}
#endif
