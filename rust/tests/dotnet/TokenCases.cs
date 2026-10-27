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

namespace TokenCases
{
    public static class Tokens
    {
        static readonly int[] Seeded = { 7, 8, 9 };

        public static string TypeName<T>() => typeof(T).Name;
        public static string ClosedName() => typeof(string).FullName;
        public static string NestedName() => typeof(Nested).Name;
        public static string ArrayName() => typeof(int[]).Name;

        public static int[] Repeated()
        {
            var copy = new int[3];
            System.Array.Copy(Seeded, copy, 3);
            return copy;
        }

        public static System.Func<int, int> StaticDelegate() => Twice;
        public static System.Func<int, int> InstanceDelegate(Instance instance) => instance.Twice;

        static int Twice(int value) => value * 2;

        public sealed class Instance
        {
            public int Twice(int value) => value * 2;
        }

        public sealed class Nested
        {
        }
    }
}
