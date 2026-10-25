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

namespace ClassCases;

public static class Empty { }
public static class EmptyGeneric<T> { }
public static class Outer
{
    public static int Value() => 17;
    public static class EmptyNested { }
    public static class Inner
    {
        public static int Value() => 23;
        public static class Leaf { public static int Value() => 31; }
    }
}
public static class OuterSuffix { public static int Value() => 99; }
public class Generic<T> { public T Echo(T value) => value; }
public static class Mixed
{
    public static int Good(int value) => unchecked(value + 1);
    public static string Unsupported(string value) => value;
}
public interface IMarker { }
public abstract class Base : IMarker { }
public sealed class Derived : Base { public int Read() => 17; }
public sealed class ExternalBase : System.Exception { }
public abstract class GenericBase<T> { }
public sealed class GenericDerived : GenericBase<int> { }
public class Fields
{
    private int storage = 3;
    public static uint Counter;
    public readonly string Label = "label";
    public const int Limit = -7;
    public const ulong Mask = ulong.MaxValue;
    public const string Text = "A\0\uD800";
    public object Item;
    public int Auto { get; set; }
    public int Read() => storage;
    public class Inner { public int Nested; }
}
public class FieldHolder<T> { public T Stored; }
[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Explicit, Size = 8)]
public struct LayoutFields
{
    [System.Runtime.InteropServices.FieldOffset(0)] public int Number;
    [System.Runtime.InteropServices.FieldOffset(4)]
    [System.Runtime.InteropServices.MarshalAs(System.Runtime.InteropServices.UnmanagedType.Bool)] public bool Ready;
}
public static class DataFields
{
    public static int[] Values() => new[] { 1, 2, 3, 4, 5, 6, 7, 8 };
}
public abstract class Methods
{
    private static int Hidden(int value) => value;
    public static int Pick(int value) => value;
    public static long Pick(long value) => value;
    public abstract int Abstract(int input);
    public static int Optional(int amount = -7) => amount;
    public static void ByRef(ref int input, out int output) { output = input; }
    [System.Runtime.InteropServices.DllImport("fixture-native", EntryPoint = "native_count")]
    public static extern int Native(int count);
}
public class Properties
{
    private int value;
    public int this[int index] { get => unchecked(value + index); set => this.value = unchecked(value - index); }
    public int Only => value;
    public int WriteOnly { set => this.value = value; }
    public static int Static => 41;
}
public class GenericProperty<T> { public T Stored { get; set; } }
