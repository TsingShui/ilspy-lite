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

[assembly: System.Runtime.Versioning.TargetFramework(".NETCoreApp,Version=v10.0")]

#if DEPENDENCY
namespace BindingLibrary;
public class Peer { }
public class Base : IFlag
{
    public Peer Stored;
    public bool Ready;
    public bool Enabled { get => Ready; set => Ready = value; }
    public virtual bool VirtualFlag => Ready;
    public bool ConstantFlag => true;
    public bool get_Pretender() => true;
    public Peer Echo(Peer value) => value;
}
public class Generic<T>
{
    public T Field;
    public T Item { get => Field; set => Field = value; }
    public T Echo(T value) => value;
    public int Echo(int value) => unchecked(value + 1);
    public U Convert<U>(U value) => value;
}
public interface IFlag { bool VirtualFlag { get; } }
public class Outer<T>
{
    public class Inner<U>
    {
        public T Left;
        public U Right;
        public T Read() => Left;
    }
}
#else
namespace BindingCases;
public class Derived : BindingLibrary.Base
{
    public bool Read() => base.Enabled;
    public void Write(bool value) => base.Enabled = value;
    public bool Pretender() => base.get_Pretender();
    public bool ReadConstant() => base.ConstantFlag;
    public bool ReadVirtual() => base.VirtualFlag;
    public override bool VirtualFlag => false;
    public BindingLibrary.Peer Relay(BindingLibrary.Peer value) => base.Echo(value);
}
public static class GenericCases
{
    public static int Read(BindingLibrary.Generic<int> value) => value.Item;
    public static int Echo(BindingLibrary.Generic<int> value, int argument) => value.Echo(argument);
    public static string Right(BindingLibrary.Outer<int>.Inner<string> value) => value.Right;
    public static int Left(BindingLibrary.Outer<int>.Inner<string> value) => value.Read();
    public static string Convert(BindingLibrary.Generic<int> value, string argument) => value.Convert(argument);
}
#endif
