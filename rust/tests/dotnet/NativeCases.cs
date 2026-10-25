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

namespace NativeCases;

public class Cells
{
    public nint Signed;
    public nuint Unsigned;
    public static nint Slot;
    public static nint Identity(nint value) => value;
    public static nuint UnsignedIdentity(nuint value) => value;
    public static nint Choose(bool first, nint left, nint right) => first ? left : right;
    public nint Replace(nint value)
    {
        nint old = Signed;
        Signed = value;
        return old;
    }
    public nuint ReplaceUnsigned(nuint value)
    {
        nuint old = Unsigned;
        Unsigned = value;
        return old;
    }
    public static nint ReplaceStatic(nint value)
    {
        nint old = Slot;
        Slot = value;
        return old;
    }
    public static nint Forward(nint value) => Identity(value);
    public static nint UnsupportedAdd(nint left, nint right) => left + right;
}
