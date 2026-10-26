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

namespace MethodInstantiationCases;

public struct Payload { public int Number; public object Reference; }
public static class Helpers
{
    public static T Identity<T>(T value) => value;
    public static T First<T, U>(T value, U other) => value;
    public static int Trace;
    public static int Argument(int divisor) { Trace++; return 100 / divisor; }
}
public class Store<T>
{
    public T Value;
    public virtual U MethodEcho<U>(T ignored, U value) => value;
    public T OwnerEcho<U>(T value, U ignored) { Value = value; return value; }
}
public class Derived<T> : Store<T>
{
    public int Calls;
    public override U MethodEcho<U>(T ignored, U value) { Calls++; return value; }
}
public class Caller<T>
{
    public U Cross<U>(Store<U> receiver, U second, T first) => receiver.OwnerEcho<T>(second, first);
}
public class NoDefault { public NoDefault(int value) { Value = value; } public int Value; }

public static class Ops
{
    public static int Int(int value) => Helpers.Identity<int>(value);
    public static string String(string value) => Helpers.Identity<string>(value);
    public static Payload Struct(Payload value) => Helpers.Identity<Payload>(value);
    public static int Distinct(int first, string second) => Helpers.First<int, string>(first, second);
    public static string Owner(Store<string> receiver, string value, int ignored) => receiver.OwnerEcho<int>(value, ignored);
    public static int Virtual(Store<string> receiver, string ignored, int value) => receiver.MethodEcho<int>(ignored, value);
    public static T Forward<T>(T value) => Helpers.Identity<T>(value);
    public static bool External(int value) => System.Array.Empty<int>() == null;
    public static object ExternalCreate() => System.Activator.CreateInstance<NoDefault>();
    public static int ArgumentOrder(Store<string> receiver, int divisor) => receiver.MethodEcho<int>("order", Helpers.Argument(divisor));
    public static string NonvirtualOrder(Store<string> receiver, int divisor) => receiver.OwnerEcho<int>("order", Helpers.Argument(divisor));
    public static string Cross(Caller<int> caller, Store<string> receiver, string second, int first) => caller.Cross<string>(receiver, second, first);
    public static int UnsupportedByRef(ref int value) => value;
}
