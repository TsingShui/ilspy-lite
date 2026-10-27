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

#if LIBRARY
namespace RelationLibrary
{
    public interface IRoot { }
    public interface ITagged { int Tag { get; } }
    public sealed class Tagged : ITagged { public int Tag => 3; }
    public struct Bits { public int Value; }
    public struct Seeded { public int Seed; public Seeded(int seed) { Seed = seed; } }
    public interface IItem<T> { }
    public class Base { public int Number; }
    public class Middle : Base { }
    public class Child : Middle, IRoot, IItem<string> { }
    public class ForeignException : System.Exception { }
    public class ForeignDerivedException : ForeignException { }
    public abstract class OpBase<TFirst, TSecond>
    {
        public readonly int Seed;
        protected OpBase(int seed) { Seed = seed; }
    }
    public class Api
    {
        public int Calls;
        public T Echo<T>(T value) => value;
        public virtual T VirtualEcho<T>(T value) { Calls++; return value; }
    }
    public class GenericBase<T> { public T Value; }
    public class GenericChild<T> : GenericBase<T>, IItem<T> { }
}
#else
namespace RelationCases
{
    public interface IItem<T> { }
    public interface IRoot { }
    public interface ISource<out T> { }
    public interface IConsumer<in T> { }
    public class Base<T> { public T Value; }
    public class Child<T> : Base<T>, IItem<T>, IRoot { }
    public class Swapped<A, B> : Base<B>, IItem<B> { }
    public sealed class Closed : Child<string> { }
    public sealed class Source : ISource<string> { }
    public sealed class Consumer : IConsumer<object> { }
    public class LocalForeign : RelationLibrary.Child { }
    public class ClosedForeign : RelationLibrary.GenericBase<string> { }
    public abstract class LocalBase<TFirst, TSecond>
    {
        public readonly int Seed;
        protected LocalBase(int seed) { Seed = seed; }
    }
    public class ForeignOuter<TKey, TValue>
    {
        public class NestedForeign : RelationLibrary.OpBase<TValue, TKey>
        {
            public NestedForeign(int seed) : base(seed) { }
        }
    }
    public class LocalOuter<TKey, TValue>
    {
        public class NestedLocal : LocalBase<TValue, TKey>
        {
            public NestedLocal(int seed) : base(seed) { }
        }
    }
    public class Cell<T>
    {
        public T Value;
        public T ReadCast(object input)
        {
            Cell<T> typed = input as Cell<T>;
            return typed == null ? default : typed.Value;
        }
    }
    public static class Ops
    {
        private static int AcceptClosedForeign(ClosedForeign input) => 37;
        public static int NullForeignFormal() => AcceptClosedForeign(null);
        public static int Pick(Base<string> input) => 11;
        public static int Pick(Child<string> input) => 29;
        public static int DeclaredOverload(Child<string> input) => Pick((Base<string>)input);
        public static Base<T> Up<T>(Child<T> input) => input;
        public static Base<B> Swap<A, B>(Swapped<A, B> input) => input;
        public static Base<string> ClosedUp(Closed input) => input;
        public static IItem<T> Interface<T>(Child<T> input) => input;
        public static IRoot Root<T>(Child<T> input) => input;
        public static IItem<B> SwappedInterface<A, B>(Swapped<A, B> input) => input;
        public static ISource<object> Covariant(ISource<string> input) => input;
        public static ISource<object> CovariantClosed(Source input) => input;
        public static IConsumer<string> Contravariant(IConsumer<object> input) => input;
        public static ISource<object> CovariantGeneric<T>(ISource<T> input) where T : class => input;
        public static RelationLibrary.Base Foreign(RelationLibrary.Child input) => input;
        public static RelationLibrary.IRoot ForeignInterface(RelationLibrary.Child input) => input;
        public static RelationLibrary.IItem<string> ForeignGenericInterface(RelationLibrary.Child input) => input;
        public static RelationLibrary.GenericBase<T> ForeignGeneric<T>(RelationLibrary.GenericChild<T> input) => input;
        public static RelationLibrary.IItem<T> ForeignOpenInterface<T>(RelationLibrary.GenericChild<T> input) => input;
        public static RelationLibrary.Base LocalToForeign(LocalForeign input) => input;

        public static int ExternalGeneric(RelationLibrary.Api api, int value) => api.Echo<int>(value);
        public static RelationLibrary.Seeded SeededValue(int seed) => new RelationLibrary.Seeded(seed);
        public static RelationLibrary.Child[] ForeignArray(int length) => new RelationLibrary.Child[length];
        public static RelationLibrary.Bits[] ForeignValueArray(int length) => new RelationLibrary.Bits[length];
        public static uint ToUInt32(long value) => unchecked((uint)value);
        public static nuint ToNativeUnsigned(int value) => unchecked((nuint)value);
        public static double DoubleSum(double a, double b) => a + b;
        public static bool DoubleLess(double a, double b) => a < b;
        public static double DoubleStore(double value) { double local = value; return local; }
        public static double DoubleFromInt(int value) => value;
        public static float DoubleToFloat(double value) => (float)value;
        public static bool FloatLe(float a, float b) => a <= b;
        public static bool FloatGe(float a, float b) => a >= b;
        public static bool FloatNotLe(float a, float b) => !(a <= b);
        public static object CastForeignInterface(RelationLibrary.Tagged value) => (RelationLibrary.ITagged)value;
        public static RelationLibrary.ITagged AsForeignInterface(object value) => value as RelationLibrary.ITagged;
        public static RelationLibrary.Tagged CastForeignClass(object value) => (RelationLibrary.Tagged)value;

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int SkipInitBranch(int value)
        {
            int result;
            if (value >= 0) result = value + 3;
            else result = value - 7;
            return result + result;
        }

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int SkipInitLoop(int limit)
        {
            int result = 0;
            for (int i = 0; i < limit; i++) result += i;
            return result;
        }

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int SkipInitBypass(int value)
        {
            if (value == 0) return 7;
            int result = value + 3;
            return result + result;
        }

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int SkipInitAddress(int value)
        {
            int result = value;
            LocalInitNegative.Increment(ref result);
            return result;
        }
    }

    public static class ForeignKeyProbe
    {
        public static RelationLibrary.Bits Field;
    }

    public static class ExternalGenericNegative
    {
        public static int Virtual(RelationLibrary.Api api, int value) => api.VirtualEcho<int>(value);
    }

    public static class LocalInitNegative
    {
        public static void Write(out int value) { value = 5; }
        public static void Increment(ref int value) { value++; }

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int AddressBeforeAssignment()
        {
            int value;
            Write(out value);
            return value;
        }

        [System.Runtime.CompilerServices.SkipLocalsInit]
        public static int ProtectedRegion(int value)
        {
            int result = value;
            try { return result; }
            finally { result++; }
        }
    }
}

namespace ExceptionRegionCases
{
    public static class Ops
    {
        public static int Trace;
        public static object ForeignCatch(System.Exception value)
        {
            try { throw value; }
            catch (RelationLibrary.ForeignDerivedException caught) { Trace = 1; return caught; }
            catch (RelationLibrary.ForeignException caught) { Trace = 2; return caught; }
        }
        public static void ForeignThrow(RelationLibrary.ForeignException value) { throw value; }
    }
}
#endif
