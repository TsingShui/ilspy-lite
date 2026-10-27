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

namespace EnumCases;

public enum S8 : sbyte { Zero = 0, Low = sbyte.MinValue, High = sbyte.MaxValue }

public class S8Ops
{
    public S8 Value;
    public static S8 Global;
    public static S8 Identity(S8 value) => value;
    public static S8 Snapshot(S8 value, S8 next) { S8 saved = value; value = next; return saved; }
    public static S8 AssignArgument(S8 value, S8 next) { value = next; return value; }
    public static S8 Choose(bool first, S8 left, S8 right) => first ? left : right;
    public S8 Read() => Value;
    public S8 Exchange(S8 next) { S8 before = Value; Value = next; return before; }
    public static S8 StaticExchange(S8 next) { S8 before = Global; Global = next; return before; }
    public static S8 Forward(S8 value) => Identity(value);
    public static S8 Zero() => 0;
    public static bool IsZero(S8 value) => value == 0;
    public static long Widen(S8 value) => unchecked((long)value);
    public static S8 SupportedOr(S8 left, S8 right) => left | right;
    public static S8 UnsupportedNarrow(int value) => unchecked((S8)value);
    public static object UnsupportedBox(S8 value) => value;
    public static S8 UnsupportedUnbox(object value) => (S8)value;
    public static S8[] UnsupportedArray(int count) => new S8[count];
}

public enum U8 : byte { Zero = 0, Low = 0, High = byte.MaxValue }

public class U8Ops
{
    public U8 Value;
    public static U8 Global;
    public static U8 Identity(U8 value) => value;
    public static U8 Snapshot(U8 value, U8 next) { U8 saved = value; value = next; return saved; }
    public static U8 AssignArgument(U8 value, U8 next) { value = next; return value; }
    public static U8 Choose(bool first, U8 left, U8 right) => first ? left : right;
    public U8 Read() => Value;
    public U8 Exchange(U8 next) { U8 before = Value; Value = next; return before; }
    public static U8 StaticExchange(U8 next) { U8 before = Global; Global = next; return before; }
    public static U8 Forward(U8 value) => Identity(value);
    public static U8 Zero() => 0;
    public static bool IsZero(U8 value) => value == 0;
    public static long Widen(U8 value) => unchecked((long)value);
    public static U8 SupportedOr(U8 left, U8 right) => left | right;
    public static U8 UnsupportedNarrow(int value) => unchecked((U8)value);
    public static object UnsupportedBox(U8 value) => value;
    public static U8 UnsupportedUnbox(object value) => (U8)value;
    public static U8[] UnsupportedArray(int count) => new U8[count];
}

public enum S16 : short { Zero = 0, Low = short.MinValue, High = short.MaxValue }

public class S16Ops
{
    public S16 Value;
    public static S16 Global;
    public static S16 Identity(S16 value) => value;
    public static S16 Snapshot(S16 value, S16 next) { S16 saved = value; value = next; return saved; }
    public static S16 AssignArgument(S16 value, S16 next) { value = next; return value; }
    public static S16 Choose(bool first, S16 left, S16 right) => first ? left : right;
    public S16 Read() => Value;
    public S16 Exchange(S16 next) { S16 before = Value; Value = next; return before; }
    public static S16 StaticExchange(S16 next) { S16 before = Global; Global = next; return before; }
    public static S16 Forward(S16 value) => Identity(value);
    public static S16 Zero() => 0;
    public static bool IsZero(S16 value) => value == 0;
    public static long Widen(S16 value) => unchecked((long)value);
    public static S16 SupportedOr(S16 left, S16 right) => left | right;
    public static S16 UnsupportedNarrow(int value) => unchecked((S16)value);
    public static object UnsupportedBox(S16 value) => value;
    public static S16 UnsupportedUnbox(object value) => (S16)value;
    public static S16[] UnsupportedArray(int count) => new S16[count];
}

public enum U16 : ushort { Zero = 0, Low = 0, High = ushort.MaxValue }

public class U16Ops
{
    public U16 Value;
    public static U16 Global;
    public static U16 Identity(U16 value) => value;
    public static U16 Snapshot(U16 value, U16 next) { U16 saved = value; value = next; return saved; }
    public static U16 AssignArgument(U16 value, U16 next) { value = next; return value; }
    public static U16 Choose(bool first, U16 left, U16 right) => first ? left : right;
    public U16 Read() => Value;
    public U16 Exchange(U16 next) { U16 before = Value; Value = next; return before; }
    public static U16 StaticExchange(U16 next) { U16 before = Global; Global = next; return before; }
    public static U16 Forward(U16 value) => Identity(value);
    public static U16 Zero() => 0;
    public static bool IsZero(U16 value) => value == 0;
    public static long Widen(U16 value) => unchecked((long)value);
    public static U16 SupportedOr(U16 left, U16 right) => left | right;
    public static U16 UnsupportedNarrow(int value) => unchecked((U16)value);
    public static object UnsupportedBox(U16 value) => value;
    public static U16 UnsupportedUnbox(object value) => (U16)value;
    public static U16[] UnsupportedArray(int count) => new U16[count];
}

public enum S32 : int { Zero = 0, Low = int.MinValue, High = int.MaxValue }

public class S32Ops
{
    public S32 Value;
    public static S32 Global;
    public static S32 Identity(S32 value) => value;
    public static S32 Snapshot(S32 value, S32 next) { S32 saved = value; value = next; return saved; }
    public static S32 AssignArgument(S32 value, S32 next) { value = next; return value; }
    public static S32 Choose(bool first, S32 left, S32 right) => first ? left : right;
    public S32 Read() => Value;
    public S32 Exchange(S32 next) { S32 before = Value; Value = next; return before; }
    public static S32 StaticExchange(S32 next) { S32 before = Global; Global = next; return before; }
    public static S32 Forward(S32 value) => Identity(value);
    public static S32 Zero() => 0;
    public static bool IsZero(S32 value) => value == 0;
    public static long Widen(S32 value) => unchecked((long)value);
    public static S32 SupportedOr(S32 left, S32 right) => left | right;
    public static object UnsupportedBox(S32 value) => value;
    public static S32 UnsupportedUnbox(object value) => (S32)value;
    public static S32[] UnsupportedArray(int count) => new S32[count];
}

public enum U32 : uint { Zero = 0, Low = 0, High = uint.MaxValue }

public class U32Ops
{
    public U32 Value;
    public static U32 Global;
    public static U32 Identity(U32 value) => value;
    public static U32 Snapshot(U32 value, U32 next) { U32 saved = value; value = next; return saved; }
    public static U32 AssignArgument(U32 value, U32 next) { value = next; return value; }
    public static U32 Choose(bool first, U32 left, U32 right) => first ? left : right;
    public U32 Read() => Value;
    public U32 Exchange(U32 next) { U32 before = Value; Value = next; return before; }
    public static U32 StaticExchange(U32 next) { U32 before = Global; Global = next; return before; }
    public static U32 Forward(U32 value) => Identity(value);
    public static U32 Zero() => 0;
    public static bool IsZero(U32 value) => value == 0;
    public static long Widen(U32 value) => unchecked((long)value);
    public static U32 SupportedOr(U32 left, U32 right) => left | right;
    public static object UnsupportedBox(U32 value) => value;
    public static U32 UnsupportedUnbox(object value) => (U32)value;
    public static U32[] UnsupportedArray(int count) => new U32[count];
}

public enum S64 : long { Zero = 0, Low = long.MinValue, High = long.MaxValue }

public class S64Ops
{
    public S64 Value;
    public static S64 Global;
    public static S64 Identity(S64 value) => value;
    public static S64 Snapshot(S64 value, S64 next) { S64 saved = value; value = next; return saved; }
    public static S64 AssignArgument(S64 value, S64 next) { value = next; return value; }
    public static S64 Choose(bool first, S64 left, S64 right) => first ? left : right;
    public S64 Read() => Value;
    public S64 Exchange(S64 next) { S64 before = Value; Value = next; return before; }
    public static S64 StaticExchange(S64 next) { S64 before = Global; Global = next; return before; }
    public static S64 Forward(S64 value) => Identity(value);
    public static S64 Zero() => 0;
    public static bool IsZero(S64 value) => value == 0;
    public static long Widen(S64 value) => unchecked((long)value);
    public static S64 SupportedOr(S64 left, S64 right) => left | right;
    public static object UnsupportedBox(S64 value) => value;
    public static S64 UnsupportedUnbox(object value) => (S64)value;
    public static S64[] UnsupportedArray(int count) => new S64[count];
}

public enum U64 : ulong { Zero = 0, Low = 0, High = ulong.MaxValue }

public class U64Ops
{
    public U64 Value;
    public static U64 Global;
    public static U64 Identity(U64 value) => value;
    public static U64 Snapshot(U64 value, U64 next) { U64 saved = value; value = next; return saved; }
    public static U64 AssignArgument(U64 value, U64 next) { value = next; return value; }
    public static U64 Choose(bool first, U64 left, U64 right) => first ? left : right;
    public U64 Read() => Value;
    public U64 Exchange(U64 next) { U64 before = Value; Value = next; return before; }
    public static U64 StaticExchange(U64 next) { U64 before = Global; Global = next; return before; }
    public static U64 Forward(U64 value) => Identity(value);
    public static U64 Zero() => 0;
    public static bool IsZero(U64 value) => value == 0;
    public static long Widen(U64 value) => unchecked((long)value);
    public static U64 SupportedOr(U64 left, U64 right) => left | right;
    public static object UnsupportedBox(U64 value) => value;
    public static U64 UnsupportedUnbox(object value) => (U64)value;
    public static U64[] UnsupportedArray(int count) => new U64[count];
}

public static class ConversionOps
{
    public static long NarrowSignedByte(int value) => unchecked((long)(sbyte)value);
    public static long NarrowUnsignedByte(int value) => unchecked((long)(byte)value);
    public static long NarrowSignedShort(int value) => unchecked((long)(short)value);
    public static long NarrowUnsignedShort(int value) => unchecked((long)(ushort)value);
    public static long NarrowSignedByte64(long value) => unchecked((long)(sbyte)value);
    public static long NarrowUnsignedByte64(long value) => unchecked((long)(byte)value);
    public static long NarrowSignedShort64(long value) => unchecked((long)(short)value);
    public static long NarrowUnsignedShort64(long value) => unchecked((long)(ushort)value);
    public static long NarrowSignedByteNative(nint value) => unchecked((long)(sbyte)value);
    public static long NarrowUnsignedByteNative(nint value) => unchecked((long)(byte)value);
    public static long NarrowSignedShortNative(nint value) => unchecked((long)(short)value);
    public static long NarrowUnsignedShortNative(nint value) => unchecked((long)(ushort)value);
}

public static class External
{
    public static DayOfWeek UnsupportedExternal(DayOfWeek value) => value;
}
