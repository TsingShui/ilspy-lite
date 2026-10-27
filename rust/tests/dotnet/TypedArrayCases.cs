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
namespace TypedArrayCases
{
    public struct Pair { public int Number; public object Tag; }
    public struct Mixed<T, U> { public T First; public U Second; }
    public class Owner<T>
    {
        public static T[] NewOwner<U>(int length) => new T[length];
        public static U[] NewMethod<U>(int length) => new U[length];
        public static Mixed<T, U>[] NewMixed<U>(int length) => new Mixed<T, U>[length];
    }
    public static class Ops
    {
        public static int Trace;
        static object Touch(object next, bool fail) { Trace = unchecked(Trace * 10 + 2); if (fail) throw null; return next; }
        public static object AddressOrder(object[] values, int index, object next, bool fail) {
            ref object cell = ref values[index];
            Trace = unchecked(Trace * 10 + 1);
            object before = cell;
            cell = Touch(next, fail);
            return before;
        }
        public static object StoreOrder(object[] values, int index, object next, bool fail) {
            Trace = unchecked(Trace * 10 + 1);
            values[index] = Touch(next, fail);
            return values[index];
        }
        public static short ReadShort(short[] values, int index) => values[index];
        public static void StoreShort(short[] values, int index, short next) { values[index] = next; }
        public static char ReadChar(char[] values, int index) => values[index];
        public static void StoreChar(char[] values, int index, char next) { values[index] = next; }
        public static bool ReadBool(bool[] values, int index) => values[index];
        public static void StoreBool(bool[] values, int index, bool next) { values[index] = next; }
        public static byte ReadByte(byte[] values, int index) => values[index];
        public static void StoreByte(byte[] values, int index, byte next) { values[index] = next; }
        public static long ReadLong(long[] values, int index) => values[index];
        public static void StoreLong(long[] values, int index, long next) { values[index] = next; }
        public static float ReadFloat(float[] values, int index) => values[index];
        public static void StoreFloat(float[] values, int index, float next) { values[index] = next; }
        public static object AddressObject(object[] values, int index, object next) { ref object cell = ref values[index]; object before = cell; cell = next; return before; }
        public static Pair AddressPair(Pair[] values, int index, Pair next) { ref Pair cell = ref values[index]; Pair before = cell; cell = next; return before; }
        public static T AddressGeneric<T>(T[] values, int index, T next) { ref T cell = ref values[index]; T before = cell; cell = next; return before; }
        public static Pair ExchangePair(Pair[] values, int index, Pair next) { Pair before = values[index]; values[index] = next; return before; }
        public static T ExchangeGeneric<T>(T[] values, int index, T next) { T before = values[index]; values[index] = next; return before; }
        public static Pair ReadPair(Pair[] values, int index) => values[index];
        public static void StorePair(Pair[] values, int index, Pair next) { values[index] = next; }
        public static T ReadGeneric<T>(T[] values, int index) => values[index];
        public static void StoreGeneric<T>(T[] values, int index, T next) { values[index] = next; }
        public static byte[] NewByte(int length) => new byte[length];
        public static object[] NewObject(int length) => new object[length];
        public static string[] NewString(int length) => new string[length];
        public static double[] NewDouble(int length) => new double[length];
        public static Pair[] NewPair(int length) => new Pair[length];
        public static T[] NewGeneric<T>(int length) => new T[length];
        public static void StoreObject(object[] values, int index, object next) { values[index] = next; }
        public static void StoreString(string[] values, int index, string next) { values[index] = next; }
        public static object ReadObject(object[] values, int index) => values[index];
        public static string ReadString(string[] values, int index) => values[index];
        public static int ByteLength(byte[] values) => values.Length;
        public static int ObjectLength(object[] values) => values.Length;
        public static int DoubleLength(double[] values) => values.Length;
        public static int PairLength(Pair[] values) => values.Length;
        public static int GenericLength<T>(T[] values) => values.Length;
    }
}
