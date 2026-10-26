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

#if DEPENDENCY
namespace StaticFieldLibrary
{
    public class Globals { public static int Number; public static object Shared; }
    public class ForeignSlots<T> { public static T Value; public static int Counter; }
    public struct ValueSlots<T> { public static int Number; }
}
#else
namespace StaticFieldCases
{
    public class ThreadSlots<T> { [ThreadStatic] public static int Number; }
    public static class NonThread { public static int Shared; }
    public class MixedSlots<T, U> { public static T Left; public static U Right; }
    public class MixedHost<T>
    {
        public static T ReadMixed<U>(U value) { MixedSlots<T, U>.Right = value; return MixedSlots<T, U>.Left; }
    }
    public class Initialized<T>
    {
        public static int Value;
        public static readonly int Readonly;
        static Initialized() { Ops.Trace = Ops.Trace * 10 + 2; Ops.Attempts++; Value = 31; Readonly = 41; if (Ops.FailInit) throw new InvalidOperationException("init"); }
    }
    public struct Pair { public int Count; public object Tag; }
    public struct LocalValueSlots<T> { public static int Number; }
    public class Slots<T>
    {
        public static T Value;
        public static int Counter;
        public static T ReadSelf() => Value;
        public static T WriteSelf(T value) { Value = value; Counter++; return Value; }
    }
    public static class Ops
    {
        public static string SignatureAnchor;
        public static int Trace;
        public static int Attempts;
        public static bool FailInit;
        public static int Argument(int value) { Trace = Trace * 10 + 1; return 100 / value; }
        public static int InitRead<T>() { Trace = Trace * 10 + 3; return Initialized<T>.Value; }
        public static int InitWrite<T>(int value) { Initialized<T>.Value = Argument(value); return Initialized<T>.Value; }
        public static int InitAddress<T>(int value) { ref int cell = ref Initialized<T>.Value; cell = Argument(value); return cell; }
        public static int InitReadonly<T>() => Initialized<T>.Readonly;
        public static int ClosedInitializerAnchor() => Initialized<int>.Readonly;
        public static int ReadInt() => Slots<int>.Value;
        public static int AddressInt(int delta) { ref int cell = ref Slots<int>.Value; cell += delta; return cell; }
        public static int WriteInt(int value) { Slots<int>.Value = value; return Slots<int>.Value; }
        public static int ReadThread<T>() => ThreadSlots<T>.Number;
        public static int WriteThread<T>(int value) { ThreadSlots<T>.Number = value; return ThreadSlots<T>.Number; }
        public static int ReadLocalValue<T>() => LocalValueSlots<T>.Number;
        public static int WriteLocalValue<T>(int value) { LocalValueSlots<T>.Number = value; return LocalValueSlots<T>.Number; }
        public static int ReadForeignValue<T>() => StaticFieldLibrary.ValueSlots<T>.Number;
        public static int WriteForeignValue<T>(int value) { StaticFieldLibrary.ValueSlots<T>.Number = value; return StaticFieldLibrary.ValueSlots<T>.Number; }
        public static int ReadForeign() => StaticFieldLibrary.Globals.Number;
        public static int WriteForeign(int value) { StaticFieldLibrary.Globals.Number = value; return StaticFieldLibrary.Globals.Number; }
        public static T ReadForeignGeneric<T>() => StaticFieldLibrary.ForeignSlots<T>.Value;
        public static T WriteForeignGeneric<T>(T value) { StaticFieldLibrary.ForeignSlots<T>.Value = value; return StaticFieldLibrary.ForeignSlots<T>.Value; }
        public static int AddressForeign(int delta) { ref int cell = ref StaticFieldLibrary.Globals.Number; cell += delta; return cell; }
        public static T Copy<T>(T value) { T before = Slots<T>.Value; Slots<T>.Value = value; return before; }
        public static T Read<T>() => Slots<T>.Value;
        public static T Write<T>(T value) { Slots<T>.Value = value; return Slots<T>.Value; }
    }
}
#endif
