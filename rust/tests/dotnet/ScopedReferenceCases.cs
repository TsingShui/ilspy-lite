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
using System.Collections.Generic;

namespace ScopedReferenceCases;

public struct Payload
{
    public int Count;
    public string Text;
    public double Bits;
    public decimal Amount;
    public object[] References;
}

public sealed class Pair<T, U>
{
    public T First;
    public U Second;
}

public static class MethodOps
{
    public static List<T> Identity<T>(List<T> value) => value;
    public static T[] Vector<T>(T[] value) => value;
    public static Dictionary<T, List<U[]>> Nested<T, U>(Dictionary<T, List<U[]>> value) => value;
    public static List<T> Choose<T>(bool first, List<T> left, List<T> right) => first ? left : right;
    public static List<T> Snapshot<T>(List<T> value)
    {
        List<T> saved = value;
        value = null;
        return saved;
    }
    public static List<T> Null<T>() => null;
    public static object Upcast<T>(T[] value) => value;
    public static List<T> AssignArgument<T>(List<T> value, List<T> next)
    {
        value = next;
        return value;
    }
    public static int Unused<T>(int value) => value;
    public static T CopyValue<T>(T value) => value;
    public static T AssignValue<T>(T value, T next) { value = next; return value; }
    public static T SnapshotValue<T>(T value, T next) { T saved = value; value = next; return saved; }
    public static T ChooseValue<T>(bool first, T left, T right) => first ? left : right;
    public static T UnsupportedBare<T>(T value) => value;
    public static object UnsupportedBox<T>(T value) => value;
    public static T[] UnsupportedAllocation<T>(int count) => new T[count];
    public static List<T> UnsupportedCall<T>(List<T> value) => Identity(value);
    public static List<T> UnsupportedConstruct<T>() => new List<T>();
}

public static class Owner<T>
{
    public static T OwnerValue(T value) => value;
    public static T MixedValue<U>(T value, U other) => value;
    public static List<T> OwnerIdentity(List<T> value) => value;
    public static T[] OwnerVector(T[] value) => value;
    public static Pair<T, U> Mixed<U>(Pair<T, U> value) => value;
    public static Pair<T, U> MixedChoose<U>(bool first, Pair<T, U> left, Pair<T, U> right) => first ? left : right;

    public static class Nested<U>
    {
        public static Pair<T, U> NestedOwner(Pair<T, U> value) => value;
        public static Pair<T, V> NestedMixed<V>(Pair<T, V> value) => value;
    }
}

public class Cell
{
    public List<T> Instance<T>(List<T> value) => value;
}

public class Receiver<T>
{
    public List<T> UnsupportedReceiver(List<T> value) => value;
    public List<T> Values;
    public static List<T> Cache;
}

public static class MemberOps
{
    public static List<T> UnsupportedField<T>(Receiver<T> value) => value.Values;
    public static List<T> UnsupportedStatic<T>() => Receiver<T>.Cache;
}
