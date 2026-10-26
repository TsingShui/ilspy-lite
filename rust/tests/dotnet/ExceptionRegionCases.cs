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

namespace ExceptionRegionCases;

public static class Ops
{
    public static int Trace;

    public static int Catch(int value)
    {
        try { return 100 / value; }
        catch (DivideByZeroException) { return -1; }
    }

    public static int FinallyOrder(int value)
    {
        try
        {
            Trace = Trace * 10 + 1;
            if (value < 0) throw (ArgumentOutOfRangeException)null;
            return value + 3;
        }
        finally { Trace = Trace * 10 + 2; }
    }

    public static int Nested(int value)
    {
        try
        {
            try
            {
                Trace = Trace * 10 + 1;
                return 100 / value;
            }
            finally { Trace = Trace * 10 + 2; }
        }
        catch (ArithmeticException)
        {
            Trace = Trace * 10 + 3;
            return -2;
        }
    }

    public static int MultiCatch(int value)
    {
        try
        {
            if (value == 0) throw (ArgumentOutOfRangeException)null;
            return 100 / value;
        }
        catch (NullReferenceException) { Trace = 1; return -3; }
        catch (ArithmeticException) { Trace = 2; return -4; }
    }

    public static object RethrowIdentity(ArgumentOutOfRangeException original)
    {
        try
        {
            try { throw original; }
            catch (ArgumentOutOfRangeException) { Trace = 1; throw; }
        }
        catch (ArgumentOutOfRangeException caught)
        {
            Trace = Trace * 10 + 2;
            return caught;
        }
    }

    public static int Rethrow(int value)
    {
        try
        {
            try { throw (ArgumentOutOfRangeException)null; }
            catch (NullReferenceException) { Trace = 1; throw; }
        }
        catch (NullReferenceException) { Trace = Trace * 10 + 2; return value; }
    }

    public static int ThrowFromFinally(int value)
    {
        try { return 100 / value; }
        finally
        {
            Trace = 7;
            if (value == 7) throw (ArgumentOutOfRangeException)null;
        }
    }

    public static int Filtered(int value)
    {
        try { return 100 / value; }
        catch (ArithmeticException) when (value == 0) { return -5; }
    }

    public static int NoException()
    {
        try { return 5; }
        catch (Exception) { return 0; }
    }
}
