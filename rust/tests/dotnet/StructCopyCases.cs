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
using System.Runtime.InteropServices;

namespace StructCopyCases;

public struct Pair { public int Number; public object Tag; }
public struct Nested { public Pair Inner; public double Bits; public object[] Items; }
[StructLayout(LayoutKind.Sequential, Pack = 1)]
public struct Packed { public byte Code; public long Wide; }
[StructLayout(LayoutKind.Explicit)]
public struct Union
{
    [FieldOffset(0)] public uint Bits;
    [FieldOffset(0)] public float Float;
}
public readonly struct Frozen
{
    public readonly long Number;
    public readonly object Tag;
    public Frozen(long number, object tag) { Number = number; Tag = tag; }
}
public struct WithCtor
{
    public int Number;
    public object Tag;
    public static int Constructions;
    public WithCtor() { Number = 123; Tag = new object(); Constructions++; }
}
public struct Empty { }

public class PairOps
{
    public Pair Value;
    public static Pair Global;
    public static Pair Identity(Pair value) => value;
    public static Pair Snapshot(Pair value, Pair next) { Pair saved = value; value = next; return saved; }
    public static Pair AssignArgument(Pair value, Pair next) { value = next; return value; }
    public static Pair Choose(bool first, Pair left, Pair right) => first ? left : right;
    public Pair Read() => Value;
    public Pair Exchange(Pair next) { Pair before = Value; Value = next; return before; }
    public static Pair StaticExchange(Pair next) { Pair before = Global; Global = next; return before; }
    public static Pair Forward(Pair value) => Identity(value);
    public static Pair ExchangeField(PairOps cell, Pair next)
    {
        Pair before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Pair UnsupportedDefault() => default;
}

public class NestedOps
{
    public Nested Value;
    public static Nested Global;
    public static Nested Identity(Nested value) => value;
    public static Nested Snapshot(Nested value, Nested next) { Nested saved = value; value = next; return saved; }
    public static Nested AssignArgument(Nested value, Nested next) { value = next; return value; }
    public static Nested Choose(bool first, Nested left, Nested right) => first ? left : right;
    public Nested Read() => Value;
    public Nested Exchange(Nested next) { Nested before = Value; Value = next; return before; }
    public static Nested StaticExchange(Nested next) { Nested before = Global; Global = next; return before; }
    public static Nested Forward(Nested value) => Identity(value);
    public static Nested ExchangeField(NestedOps cell, Nested next)
    {
        Nested before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Nested UnsupportedDefault() => default;
}

public class PackedOps
{
    public Packed Value;
    public static Packed Global;
    public static Packed Identity(Packed value) => value;
    public static Packed Snapshot(Packed value, Packed next) { Packed saved = value; value = next; return saved; }
    public static Packed AssignArgument(Packed value, Packed next) { value = next; return value; }
    public static Packed Choose(bool first, Packed left, Packed right) => first ? left : right;
    public Packed Read() => Value;
    public Packed Exchange(Packed next) { Packed before = Value; Value = next; return before; }
    public static Packed StaticExchange(Packed next) { Packed before = Global; Global = next; return before; }
    public static Packed Forward(Packed value) => Identity(value);
    public static Packed ExchangeField(PackedOps cell, Packed next)
    {
        Packed before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Packed UnsupportedDefault() => default;
}

public class UnionOps
{
    public Union Value;
    public static Union Global;
    public static Union Identity(Union value) => value;
    public static Union Snapshot(Union value, Union next) { Union saved = value; value = next; return saved; }
    public static Union AssignArgument(Union value, Union next) { value = next; return value; }
    public static Union Choose(bool first, Union left, Union right) => first ? left : right;
    public Union Read() => Value;
    public Union Exchange(Union next) { Union before = Value; Value = next; return before; }
    public static Union StaticExchange(Union next) { Union before = Global; Global = next; return before; }
    public static Union Forward(Union value) => Identity(value);
    public static Union ExchangeField(UnionOps cell, Union next)
    {
        Union before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Union UnsupportedDefault() => default;
}

public class FrozenOps
{
    public Frozen Value;
    public static Frozen Global;
    public static Frozen Identity(Frozen value) => value;
    public static Frozen Snapshot(Frozen value, Frozen next) { Frozen saved = value; value = next; return saved; }
    public static Frozen AssignArgument(Frozen value, Frozen next) { value = next; return value; }
    public static Frozen Choose(bool first, Frozen left, Frozen right) => first ? left : right;
    public Frozen Read() => Value;
    public Frozen Exchange(Frozen next) { Frozen before = Value; Value = next; return before; }
    public static Frozen StaticExchange(Frozen next) { Frozen before = Global; Global = next; return before; }
    public static Frozen Forward(Frozen value) => Identity(value);
    public static Frozen ExchangeField(FrozenOps cell, Frozen next)
    {
        Frozen before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Frozen UnsupportedDefault() => default;
}

public class WithCtorOps
{
    public WithCtor Value;
    public static WithCtor Global;
    public static WithCtor Identity(WithCtor value) => value;
    public static WithCtor Snapshot(WithCtor value, WithCtor next) { WithCtor saved = value; value = next; return saved; }
    public static WithCtor AssignArgument(WithCtor value, WithCtor next) { value = next; return value; }
    public static WithCtor Choose(bool first, WithCtor left, WithCtor right) => first ? left : right;
    public WithCtor Read() => Value;
    public WithCtor Exchange(WithCtor next) { WithCtor before = Value; Value = next; return before; }
    public static WithCtor StaticExchange(WithCtor next) { WithCtor before = Global; Global = next; return before; }
    public static WithCtor Forward(WithCtor value) => Identity(value);
    public static WithCtor ExchangeField(WithCtorOps cell, WithCtor next)
    {
        WithCtor before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static WithCtor UnsupportedDefault() => default;
}

public class EmptyOps
{
    public Empty Value;
    public static Empty Global;
    public static Empty Identity(Empty value) => value;
    public static Empty Snapshot(Empty value, Empty next) { Empty saved = value; value = next; return saved; }
    public static Empty AssignArgument(Empty value, Empty next) { value = next; return value; }
    public static Empty Choose(bool first, Empty left, Empty right) => first ? left : right;
    public Empty Read() => Value;
    public Empty Exchange(Empty next) { Empty before = Value; Value = next; return before; }
    public static Empty StaticExchange(Empty next) { Empty before = Global; Global = next; return before; }
    public static Empty Forward(Empty value) => Identity(value);
    public static Empty ExchangeField(EmptyOps cell, Empty next)
    {
        Empty before = cell.Value;
        cell.Value = next;
        return before;
    }
    public static Empty UnsupportedDefault() => default;
}

public ref struct Borrowed { public Span<int> Values; }
public struct GenericPair<T> { public T Value; }

public static class UnsupportedOps
{
    public static Guid UnsupportedExternal(Guid value) => value;
    public static Pair? UnsupportedNullable(Pair? value) => value;
    public static int UnsupportedMember(Pair value) => value.Number;
    public static Pair UnsupportedNew(int value) => new Pair { Number = value };
    public static object UnsupportedBox(Pair value) => value;
    public static Pair UnsupportedByRef(ref Pair value) => value;
    public static Borrowed RecoveredRefLike(Borrowed value) => value;
    public static GenericPair<T> UnsupportedConstructed<T>(GenericPair<T> value) => value;
}
