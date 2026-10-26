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
using System.IO;

namespace ExceptionTypeCases
{
    public class Custom : Exception
    {
        public Custom(string message, Exception inner) : base(message, inner) { Ops.Trace = Ops.Trace * 10 + 3; }
    }
    public sealed class Derived : Custom
    {
        public Derived(string message, Exception inner) : base(message, inner) { Ops.Trace = Ops.Trace * 10 + 4; }
    }
    public static class Ops
    {
        public static int Trace;
        public static void ThrowBase(Exception value) { throw value; }
        public static void ThrowSystem(InvalidOperationException value) { throw value; }
        public static object Rethrow(Exception value)
        {
            try { try { throw value; } catch (Exception) { Trace = 7; throw; } }
            catch (Exception caught) { return caught; }
        }
        public static Exception Construct(string message, Exception inner) => new InvalidOperationException(message, inner);
        public static Exception Range(string parameter, object actual, string message) => new ArgumentOutOfRangeException(parameter, actual, message);
        public static void ThrowLocal(Custom value) { throw value; }
        public static object MatchLocal(Custom value)
        {
            try { throw value; }
            catch (Derived caught) { Trace = 1; return caught; }
            catch (Custom caught) { Trace = 2; return caught; }
            finally { Trace = Trace * 10 + 5; }
        }
        public static Exception ConstructLocal(string message, Exception inner) => new Derived(message, inner);
        public static string TextArg(int digit, string value, int fail)
        { Trace = Trace * 10 + digit; if (fail == digit) throw new FormatException(value); return value; }
        public static Exception InnerArg(Exception value, int fail)
        { Trace = Trace * 10 + 3; if (fail == 3) throw value; return value; }
        public static Exception Order(string message, string parameter, Exception inner, int fail) => new ArgumentException(TextArg(1, message, fail), TextArg(2, parameter, fail), InnerArg(inner, fail));
        public static Exception Io(string message, Exception inner) => new IOException(message, inner);
        public static Exception Format(string message) => new FormatException(message);
        public static object CatchIo(InvalidDataException value)
        {
            try { throw value; }
            catch (InvalidDataException caught) { return caught; }
        }
    }
}
