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

using System;

namespace ExceptionCases;

public class Cells
{
    public int Effects;
    public static ArgumentOutOfRangeException Make(string parameter, string message) => new ArgumentOutOfRangeException(parameter, message);
    public static ArgumentOutOfRangeException Literal() => new ArgumentOutOfRangeException("parameter", "\ud800\0details");
    public static ArgumentOutOfRangeException Null() => null;
    public ArgumentOutOfRangeException Build(string parameter, string message)
    {
        Effects = 1;
        var result = new ArgumentOutOfRangeException(parameter, message);
        Effects = 2;
        return result;
    }
    public void Discard(string parameter, string message)
    {
        Effects = 1;
        _ = new ArgumentOutOfRangeException(parameter, message);
        Effects = 2;
    }
    public static ArgumentOutOfRangeException UnsupportedOverload(string parameter) => new ArgumentOutOfRangeException(parameter);
    public static Exception UnsupportedType(string message) => new Exception(message);
}
