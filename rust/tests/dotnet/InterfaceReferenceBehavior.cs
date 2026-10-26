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
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    static void InterfaceReferenceMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned interface fixture and carrier.");
        Type Named(string name) => fixture.GetType("InterfaceReferenceCases." + name)
            ?? throw new Exception("Owned interface type missing: " + name);
        Type ops = Named("Ops");
        Type valueType = Named("IValue");
        Type childType = Named("IChild");
        Type counter = Named("Counter");
        Type derived = Named("DerivedCounter");
        Type holder = Named("Holder");
        object Make(Type type) => Activator.CreateInstance(type);
        object MakeHolder()
        {
            object result = Make(holder);
            holder.GetField("Field").SetValue(result, Make(counter));
            return result;
        }
        object[] Candidates(Type type)
        {
            if (type == typeof(int)) return new object[] { 0, 2, -2 };
            if (type == typeof(object)) return new[] { null, Make(counter), Make(Named("Explicit")),
                Make(Named("Boxed")), Make(Named("Unrelated")), "text" };
            if (type == valueType) return new[] { null, Make(counter), Make(derived), Make(Named("Explicit")), Make(Named("Boxed")) };
            if (type == childType || type == counter) return new[] { null, Make(counter), Make(derived) };
            if (type == derived) return new[] { null, Make(derived) };
            if (type == holder) return new[] { null, MakeHolder(), Make(holder) };
            throw new Exception("Unexpected interface parameter: " + type);
        }
        (object[] Arguments, List<object> Nodes) Clone(object[] arguments)
        {
            var copies = new Dictionary<object, object>(ReferenceEqualityComparer.Instance);
            var nodes = new List<object>();
            object Copy(object value)
            {
                if (value == null || value is string || value.GetType().IsPrimitive) return value;
                if (copies.TryGetValue(value, out object known)) return known;
                object result = Make(value.GetType());
                copies.Add(value, result);
                nodes.Add(result);
                foreach (FieldInfo field in value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance))
                    field.SetValue(result, Copy(field.GetValue(value)));
                return result;
            }
            return (arguments.Select(Copy).ToArray(), nodes);
        }
        string Describe(object value, List<object> nodes)
        {
            if (value == null) return "null";
            if (value.GetType().IsPrimitive) return value.GetType().FullName + ":" + value;
            int identity = nodes.FindIndex(node => ReferenceEquals(node, value));
            return identity >= 0 ? "node:" + identity : "new:" + value.GetType().FullName;
        }
        string State(List<object> nodes) => string.Join("|", nodes.Select(node => node.GetType().FullName + ":"
            + string.Join(",", node.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance)
                .Select(field => field.Name + "=" + Describe(field.GetValue(node), nodes)))));

        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("InterfaceReferenceCases.Recovered")
            ?? throw new Exception("Recovered interface carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (selected.Length == 0) throw new Exception("Owned interface selection is empty.");
        int comparisons = 0;
        foreach (MethodInfo lowered in selected.OrderBy(m => m.Name))
        {
            MethodInfo original = ops.GetMethod(lowered.Name) ?? throw new Exception("Owned counterpart missing.");
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (original.ReturnType != lowered.ReturnType
                || !parameters.SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                throw new Exception("Recovered interface signature differs: " + original.Name);
            var cases = new List<object[]> { Array.Empty<object>() };
            foreach (Type parameter in parameters)
                cases = cases.SelectMany(prefix => Candidates(parameter)
                    .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
            foreach (object[] arguments in cases)
            {
                var beforeArgs = Clone(arguments);
                var afterArgs = Clone(arguments);
                Outcome before = Capture(() => OriginalCall(original)(null, beforeArgs.Arguments), new List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, afterArgs.Arguments), new List<State>());
                string expected = Describe(before.Value, beforeArgs.Nodes);
                string actual = Describe(after.Value, afterArgs.Nodes);
                if (before.Error != after.Error || expected != actual || State(beforeArgs.Nodes) != State(afterArgs.Nodes))
                    throw new Exception("Recovered interface differs: " + original.Name
                        + " expected=" + (before.Error?.Name ?? expected)
                        + " actual=" + (after.Error?.Name ?? actual)
                        + " state=" + State(beforeArgs.Nodes) + "/" + State(afterArgs.Nodes));
                comparisons++;
            }
        }
        Console.WriteLine("Interface reference behavior: " + comparisons + " comparisons, " + selected.Length
            + " methods; identity, explicit and boxed dispatch, state, casts and argument exception order match.");
    }
}
