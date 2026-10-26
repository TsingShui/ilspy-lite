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

namespace ManagedLocationCases;

public struct Payload { public int Number; public object Reference; }
public enum Kind : short { Zero, One, Two }
public class Cell { public int Number; public string Text; public Payload Data; }
public class ReadOnlyCell { public readonly Payload Data = new Payload { Number = 19 }; }
public struct Envelope { public readonly Payload Data; public Envelope(Payload data) { Data = data; } }

public static class Ops
{
    public static int Trace;
    public static int GlobalNumber;
    public static Cell Global;
    public static readonly Payload ReadOnlyPayload = new Payload { Number = 37 };

    public static int Update(ref int value, int next) { int before = value; value = next; return before; }
    public static int Aliased(ref int first, ref int second) { first++; second += 10; return first; }
    public static string Text(ref string value, string next) { string before = value; value = next; return before; }
    public static sbyte SignedByte(ref sbyte value, sbyte next) { sbyte before = value; value = next; return before; }
    public static char Character(ref char value, char next) { char before = value; value = next; return before; }
    public static nint Native(ref nint value, nint next) { nint before = value; value = next; return before; }
    public static nuint UnsignedNative(ref nuint value, nuint next) { nuint before = value; value = next; return before; }
    public static float Single(ref float value, float next) { float before = value; value = next; return before; }
    public static object BooleanBox(ref bool value) => value;
    public static int BooleanBits(ref bool value) => value ? 1 : 0;
    public static bool Boolean(ref bool value, bool next) { bool before = value; value = next; return before; }
    public static byte Byte(ref byte value, byte next) { byte before = value; value = next; return before; }
    public static ushort Char(ref ushort value, ushort next) { ushort before = value; value = next; return before; }
    public static ulong Wide(ref ulong value, ulong next) { ulong before = value; value = next; return before; }
    public static void Flag(out bool value) { value = true; }
    public static Kind Enum(ref Kind value, Kind next) { Kind before = value; value = next; return before; }
    public static Payload Struct(ref Payload value, Payload next) { Payload before = value; value = next; return before; }
    public static void Copy(ref Payload destination, ref Payload source) { destination = source; }
    public static T Generic<T>(ref T value, T next) { T before = value; value = next; return before; }
    public static void Default<T>(out T value) { value = default; }

    public static int Effect(int divisor) { Trace++; return 100 / divisor; }
    public static int ReadonlyInstance(ReadOnlyCell cell) => cell.Data.Number;
    public static int ReadonlyNested(ref Envelope envelope) => envelope.Data.Number;
    public static int ReadonlyStatic() => ReadOnlyPayload.Number;
    public static int Field(Cell cell, int divisor) => Update(ref cell.Number, Effect(divisor));
    public static int Array(int[] values, int index, int divisor) => Update(ref values[index], Effect(divisor));
    public static int Static(int divisor) => Update(ref GlobalNumber, Effect(divisor));
    public static int Replace(Cell next) { Global = next; return 23; }
    public static int Captured(Cell next) => Update(ref Global.Number, Replace(next));
    public static int ThrowAfterWrite(ref int value, int divisor) { value = 17; return Effect(divisor); }
    public static string ScalarReceiver(int value) => value.ToString();
    public static int ScalarLocal(int next) { int local = 7; return Update(ref local, next) + local; }
    public static int AliasedCaller(int value) => Aliased(ref value, ref value);
    public static Payload DefaultPayload() { Payload value = default; return value; }

    public static ref int UnsupportedReturn(ref int value) => ref value;
    public static int UnsupportedJoin(bool first, ref int left, ref int right) => Update(ref (first ? ref left : ref right), 29);
    public static int UnsupportedRefLocal(ref int value) { ref int slot = ref value; slot++; return value; }
}
