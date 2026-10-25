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

namespace StringCases;

public class Cells
{
    public string Text;
    public int Effects;
    public static string Empty() => "";
    public static string Ascii() => "plain-text";
    public static string Escapes() => "\0\"\\\r\n\t\b\f\v\u001f\u007f";
    public static string Units() => "\ud800X\udfff\ud83d\ude00\u4e2d\uffff";
    public static string Shared() => "same";
    public static string SharedAgain() => "same";
    public static string Identity(string value) => value;
    public static string Choose(bool first, string left, string right) => first ? left : right;
    public static string Null() => null;
    public string Replace(string replacement)
    {
        string old = Text;
        Text = replacement;
        return old;
    }
    public string SetLiteral()
    {
        Effects = 1;
        Text = "\ud800\0";
        Effects = 2;
        return Text;
    }
}
