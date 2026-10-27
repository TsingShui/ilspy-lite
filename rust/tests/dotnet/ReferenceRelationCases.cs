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

#if !LIBRARY
extern alias OtherRelation;
#endif

#if LIBRARY
namespace RelationLibrary
{
    public interface IRoot { }
    public interface ITagged { int Tag { get; } }
    public interface IView<out T> { }
    public class VisibleBase { }
    public class VisibleGeneric<T> { }
    public sealed class Hidden : VisibleBase { }
    public sealed class HiddenBox<T> : VisibleGeneric<T> { }
    public class HiddenChild : IView<Hidden> { }
    public class HiddenGeneric<T> : IView<HiddenBox<T>> { }
    public class Deep0 { }
    public class Deep1 : Deep0 { }
    public class Deep2 : Deep1 { }
    public class Deep3 : Deep2 { }
    public class Deep4 : Deep3 { }
    public class Deep5 : Deep4 { }
    public class Deep6 : Deep5 { }
    public class Deep7 : Deep6 { }
    public class Deep8 : Deep7 { }
    public class Deep9 : Deep8 { }
    public class Deep10 : Deep9 { }
    public class Deep11 : Deep10 { }
    public class Deep12 : Deep11 { }
    public sealed class DeepValue : Deep12 { }
    public sealed class Tagged : ITagged { public int Tag => 3; }
    public struct Bits { public int Value; }
    public struct Seeded { public int Seed; public Seeded(int seed) { Seed = seed; } }
    public struct Holder<T> { public T Value; }
    public struct SeededBox { public int Seed; public T Echo<T>(T input) { Seed = 1; return input; } }
    public ref struct RefBox { public int Value; public RefBox(int value) { Value = value; } public int Read() => Value; }
    public interface IItem<T> { }
    public class Base { public int Number; }
    public class Middle : Base { }
    public class Child : Middle, IRoot, IItem<string> { }
    public class ForeignException : System.Exception { }
    public class ForeignDerivedException : ForeignException { }
    public static class ConstructorTrace { public static string Log = ""; }
    public abstract class OpBase<TFirst, TSecond>
    {
        public readonly int Seed;
        protected OpBase(int seed)
        {
            ConstructorTrace.Log += typeof(TFirst).Name + ":" + typeof(TSecond).Name + ":" + seed + ";";
            if (seed == -17) throw new System.InvalidOperationException("owned constructor failure");
            Seed = seed;
        }
    }
    public class GenericCtorBase<TFirst, TSecond>
    {
        public readonly TFirst First;
        public readonly TSecond Second;
        protected GenericCtorBase(TFirst first, TSecond second)
        {
            ConstructorTrace.Log += typeof(TFirst).Name + ":" + typeof(TSecond).Name + ";";
            First = first;
            Second = second;
        }
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
    public struct LocalBox { public int Seed; public T Echo<T>(T input) { Seed = 1; return input; } }
    public struct LocalBox2<T> { public T Value; public U Echo<U>(U input) => input; }
    public sealed class VolatileHolder { public volatile int Value; public volatile object Reference = ""; }
    public static class VolatileStore { public static volatile int Shared; }
    public interface ITransform<T> { T Apply(T value); }
    public sealed class UpperTransform : ITransform<string>
    {
        public string Apply(string value) => value;
    }

    public interface IRoot { }
    public interface ISource<out T> { }
    public interface IConsumer<in T> { }
    public class Base<T> { public T Value; }
    public class Child<T> : Base<T>, IItem<T>, IRoot { }
    public class Swapped<A, B> : Base<B>, IItem<B> { }
    public sealed class Closed : Child<string> { }
    public sealed class ClosedSiblingA : Base<string> { }
    public sealed class ClosedSiblingB : Base<string> { }
    public sealed class ClosedSiblingWrong : Base<object> { }
    public sealed class ForeignSiblingA : RelationLibrary.GenericBase<string> { }
    public sealed class ForeignSiblingB : RelationLibrary.GenericBase<string> { }
    public sealed class ForeignSiblingWrong : RelationLibrary.GenericBase<object> { }
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
            public int Derived;
            public NestedForeign(int seed) : base(seed) { Derived = seed + 5; }
        }
        public class NestedForeignIdentity : RelationLibrary.OpBase<TKey, TValue>
        {
            public NestedForeignIdentity(int seed) : base(seed) { }
        }
        public class NestedTyped : RelationLibrary.GenericCtorBase<TValue, TKey>
        {
            public NestedTyped(TValue first, TKey second) : base(first, second) { }
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
    public static class RefHelpers
    {
        public static ref int First(int[] values) => ref values[0];
    }
    public static class ValueSpecHelper
    {
        public static int Target<T>() => typeof(T) == typeof(RelationLibrary.Holder<string>) ? 37 : 29;
    }
    public static class AlternateReference
    {
        public static OtherRelation::RelationLibrary.VisibleBase Identity(OtherRelation::RelationLibrary.VisibleBase value) => value;
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
        public static Base<string> ClosedSiblingJoin(int choose, ClosedSiblingA first, ClosedSiblingB second)
            => choose != 0 ? first : second;
        public static bool SiblingJoinIsNull(int choose, ClosedSiblingA first, ClosedSiblingB second)
            => (choose != 0 ? (Base<string>)first : second) is null;
        public static RelationLibrary.GenericBase<string> ForeignSiblingJoin(int choose, ForeignSiblingA first, ForeignSiblingB second)
            => choose != 0 ? first : second;
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
        public static RelationLibrary.IView<RelationLibrary.VisibleBase> ForeignHiddenToBase(RelationLibrary.HiddenChild input) => input;
        public static RelationLibrary.IView<RelationLibrary.VisibleGeneric<A>> ForeignHiddenOpen<A, B>(RelationLibrary.HiddenGeneric<A> input) => input;
        public static RelationLibrary.Deep0 ForeignDeep(RelationLibrary.DeepValue input) => input;
        public static RelationLibrary.Base LocalToForeign(LocalForeign input) => input;

        public static int ExternalGeneric(RelationLibrary.Api api, int value) => api.Echo<int>(value);
        public static int ReadFirst(int[] values) => RefHelpers.First(values);
        public static int ReadVolatile(VolatileHolder holder) => holder.Value;
        public static int WriteVolatile(VolatileHolder holder, int value) { holder.Value = value; return value; }
        public static bool VolatileReferenceNull(VolatileHolder holder) => holder.Reference == null;
        public static int ReadVolatileStatic() => VolatileStore.Shared;
        public static void WriteVolatileStatic(int value) => VolatileStore.Shared = value;
        public static int ReadRefBox(RelationLibrary.RefBox box) => box.Read();
        public static int CopyRefBox(RelationLibrary.RefBox box) { RelationLibrary.RefBox copy = box; return copy.Value; }
        public static int CallValueGeneric(RelationLibrary.SeededBox box, int value) => box.Echo(value);
        public static string CallLocalValueGeneric(LocalBox box, string value) => box.Echo(value);
        public static int CallConstructedValueGeneric(LocalBox2<int> box, int value) => box.Echo(value);
        public static RelationLibrary.IRoot AsRoot(RelationLibrary.IRoot value) => value;
        public static RelationLibrary.IRoot PassRoot(LocalForeign value) => AsRoot(value);
        public static RelationLibrary.IItem<string> ToItem(RelationLibrary.GenericChild<string> value) => value;
        public static RelationLibrary.IItem<string> PassItem(RelationLibrary.GenericChild<string> value) => ToItem(value);
        public static RelationLibrary.Middle TakeMiddle(RelationLibrary.Middle value) => value;
        public static RelationLibrary.Middle PassMiddle(RelationLibrary.Child value) => TakeMiddle(value);
        public static string ApplyLocalInterface(ITransform<string> transform, string value) => transform.Apply(value);
        public static string ReadForeignValue(RelationLibrary.GenericBase<string> box) => box.Value;
        public static int ReadForeignHolder(RelationLibrary.Holder<int> holder) => holder.Value;
        public static int ForeignValueArgumentOnly() => ValueSpecHelper.Target<RelationLibrary.Holder<string>>();
        public static void WriteForeignValue(RelationLibrary.GenericBase<string> box, string value) => box.Value = value;
        public static object BoxDouble(double value) => value;
        public static object BoxSingle(float value) => value;
        public static double UnboxDouble(object value) => (double)value;
        public static unsafe int ReadPointer(int* pointer) => *pointer;
        public static unsafe int ReadWrite(int* pointer) { int value = *pointer; *pointer = value + 1; return value; }
        public static unsafe bool SamePointer(int* first, int* second) => first == second;
        public static unsafe bool NullPointer(int* pointer) => pointer == null;
        public static unsafe int OffsetValue(int* pointer, int offset) => *(pointer + offset);
        public static unsafe nint Difference(int* first, int* second) => (nint)(first - second);
        public static bool SameNative(nint first, nint second) => first == second;
        public static nint AddNative(nint first, nint second) => unchecked(first + second);
        public static bool SameReference(RelationLibrary.Base first, object second) => first == second;
        public static bool DifferentReference(RelationLibrary.Base first, object second) => first != second;
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
