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
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    static void ValueReceiverMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned value-receiver carrier.");
        var generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        var carrier = generated.GetType("VirtualCallCases.RecoveredReceivers") ?? throw new Exception("Receiver carrier missing.");
        int comparisons = 0;
        foreach (string name in new[] { "Shape", "Value" })
        {
            Type owner = fixture.GetType("VirtualCallCases." + name) ?? throw new Exception("Owned receiver missing.");
            MethodInfo original = owner.GetMethod(name == "Shape" ? "Describe" : "ToString", BindingFlags.Public | BindingFlags.Instance | BindingFlags.DeclaredOnly);
            MethodInfo lowered = carrier.GetMethod(name + "_Describe") ?? throw new Exception("Recovered receiver method missing.");
            Type receiverType = name == "Shape" ? fixture.GetType("VirtualCallCases.Square") : owner;
            FieldInfo member = receiverType.GetFields(BindingFlags.Public | BindingFlags.Instance).Single();
            foreach (int number in new[] { 0, 7, int.MinValue })
            foreach (bool isNull in name == "Shape" ? new[] { false, true } : new[] { false })
            {
                object receiver = isNull ? null : Activator.CreateInstance(receiverType);
                if (receiver != null) member.SetValue(receiver, number);
                object InvokeLowered() => name == "Value"
                    ? ReceiverCall(lowered, true, true)(receiver, Array.Empty<object>())
                    : OriginalCall(lowered)(null, new[] { receiver });
                Outcome before = Capture(() => ReceiverCall(original, false, false)(receiver, Array.Empty<object>()), new System.Collections.Generic.List<State>());
                Outcome after = Capture(InvokeLowered, new System.Collections.Generic.List<State>());
                Outcome originalAgain = Capture(() => ReceiverCall(original, false, false)(receiver, Array.Empty<object>()), new System.Collections.Generic.List<State>());
                Outcome loweredAgain = Capture(InvokeLowered, new System.Collections.Generic.List<State>());
                if (before.Error != after.Error || !Equals(before.Value, after.Value)
                    || originalAgain.Error != loweredAgain.Error || !Equals(originalAgain.Value, loweredAgain.Value)
                    || (before.Error == null && ReferenceEquals(before.Value, originalAgain.Value) != ReferenceEquals(after.Value, loweredAgain.Value))
                    || (receiver != null && !Equals(member.GetValue(receiver), number)))
                    throw new Exception("Recovered value receiver differs: " + name);
                comparisons++;
            }
        }
        Console.WriteLine("Value receiver behavior: " + comparisons + " comparisons, 2 recovered methods; direct base/value receivers, virtual inner dispatch, signed values, null checks and result freshness match.");
    }

    static void VirtualMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("VirtualCallCases.Ops") == null)
            throw new Exception("Expected owned virtual-call fixture.");
        Type shape = fixture.GetType("VirtualCallCases.Shape");
        Type square = fixture.GetType("VirtualCallCases.Square");
        Type ops = fixture.GetType("VirtualCallCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("VirtualCallCases.Recovered")
            ?? throw new Exception("Recovered virtual-call carrier missing.");

        object MakeSquare(int size)
        {
            object value = Activator.CreateInstance(square);
            square.GetField("Size").SetValue(value, size);
            return value;
        }

        // The carrier holds exactly the bodies the production scan recovered;
        // the driver selects them from scan output, so this list cannot drift.
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned virtual-call selection is empty.");

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered virtual-call method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered virtual-call signature differs: " + original.Name);
            object[] shapes = { MakeSquare(3), MakeSquare(0), MakeSquare(-2), null };
            // Candidates follow the declared parameter type. Passing a Shape to an
            // int parameter only compared two invocation mechanisms (delegate unbox
            // against reflection binding), which is not a property of the method.
            object[] Candidates(Type type)
            {
                if (type == typeof(int)) return new object[] { 3, 0, -2 };
                if (type == shape) return shapes;
                if (type == typeof(System.Collections.Generic.IEnumerable<int>))
                    return new object[] { null, Array.Empty<int>(), new[] { 1, 2, 3 }, new System.Collections.Generic.List<int> { 7 } };
                if (type == typeof(string)) return new object[] { "x", null };
                if (type.FullName == "VirtualCallCases.Value") return new[] { int.MinValue, -2, 3 }.Select(number =>
                {
                    object value = Activator.CreateInstance(type);
                    type.GetFields(BindingFlags.Public | BindingFlags.Instance).Single().SetValue(value, number);
                    return value;
                }).ToArray();
                if (!type.IsValueType) return new object[] { null };
                throw new Exception("Unexpected owned virtual-call parameter type: " + type);
            }
            var cases = new System.Collections.Generic.List<object[]>();
            if (parameters.Length == 1) foreach (object value in Candidates(parameters[0])) cases.Add(new[] { value });
            else if (parameters.Length == 2)
                foreach (object first in Candidates(parameters[0]))
                    foreach (object second in Candidates(parameters[1]))
                        cases.Add(new[] { first, second });
            else throw new Exception("Unexpected owned virtual-call arity.");
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                bool sameResult = original.Name == "UnsupportedConstrained" ? Equals(before.Value, after.Value) : Same(before.Value, after.Value);
                if (before.Error != after.Error || !sameResult)
                    throw new Exception("Recovered virtual dispatch differs: " + original.Name
                        + " args=(" + string.Join(",", arguments.Select(a => a == null ? "null" : a.GetType().Name)) + ")"
                        + " expected=" + (before.Error?.Name ?? before.Value?.ToString())
                        + " actual=" + (after.Error?.Name ?? after.Value?.ToString()));
                if (original.Name == "UnsupportedConstrained" && before.Error == null)
                {
                    Outcome beforeAgain = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                    Outcome afterAgain = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                    if (beforeAgain.Error != afterAgain.Error || !Equals(beforeAgain.Value, afterAgain.Value)
                        || ReferenceEquals(before.Value, beforeAgain.Value) != ReferenceEquals(after.Value, afterAgain.Value))
                        throw new Exception("Recovered constrained string freshness differs.");
                    checks++;
                }
                checks++;
            }
        }
        Console.WriteLine($"Virtual call behavior: {checks} comparisons, {selected.Length} recovered methods; virtual dispatch, null-check exceptions, argument order and results match.");
    }
}
