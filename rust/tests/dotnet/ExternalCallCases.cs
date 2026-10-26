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

#if DEPENDENCY
namespace ExternalCalls;
public class Service
{
    public int Value;
    public virtual int Virtual() => 1;
    public int Direct() => 2;
    public virtual int Echo(int value) => value + 1;
    public virtual string Describe() => "service";
    public T EchoOf<T>(T value) => value;
}
public sealed class Sealed
{
    public int Only() => 7;
}
public abstract class Abstract
{
    public abstract int Compute();
}
public interface IWidget
{
    int Measure();
}
public class Gauge : IWidget
{
    public int Measure() => 11;
}
public class DerivedService : Service
{
    public override int Virtual() => 99;
    public override string Describe() => "derived";
}
public class Concrete : Abstract
{
    public override int Compute() => 3;
}
#else
using ExternalCalls;
namespace ExternalCallCases;
public static class Ops
{
    public static int CallVirtual(Service service) => service.Virtual();
    public static int CallDirect(Service service) => service.Direct();
    public static int CallEcho(Service service, int value) => service.Echo(value);
    public static string CallDescribe(Service service) => service.Describe();
    public static int CallSealed(Sealed sealedValue) => sealedValue.Only();
    public static int CallTwice(Service service) => service.Virtual() + service.Direct();
    public static int CallAbstract(Abstract abstractValue) => abstractValue.Compute();
    public static int UnsupportedInterface(IWidget widget) => widget.Measure();
    public static int UnsupportedGeneric(Service service) => service.EchoOf(5);
}
#endif
