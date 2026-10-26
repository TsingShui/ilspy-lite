using System;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    // A result is described by its value, not by identity: the point of this
    // milestone is that the call reaches the same implementation, not that the
    // same instance comes back.
    static string ExternalCallDescriptor(object value)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        if (type.IsPrimitive || value is string)
            return "value:" + Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        return "type:" + type.Name;
    }

    static void ExternalCallMain(string[] args)
    {
        if (args.Length != 4 || fixture.GetType("ExternalCallCases.Ops") == null)
            throw new Exception("Expected owned external-call fixture and its library.");
        string libraryPath = System.IO.Path.GetFullPath(args[3]);
        Assembly resolved = null;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
        {
            if (name.Name == "ExternalCallLibrary") return resolved ??= context.LoadFromAssemblyPath(libraryPath);
            return null;
        };
        resolved = AssemblyLoadContext.Default.LoadFromAssemblyPath(libraryPath);
        Type ops = fixture.GetType("ExternalCallCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExternalCallCases.Recovered")
            ?? throw new Exception("Recovered external-call carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned external-call selection is empty.");

        object[] Candidates(Type type)
        {
            if (type == typeof(int)) return new object[] { 0, 1, -1 };
            if (type == typeof(string)) return new object[] { null, "x" };
            if (type.IsValueType) return new object[] { Activator.CreateInstance(type) };
            // Reference parameters get the base instance, a derived instance (so a
            // devirtualized call must differ), and null (so the null check is kept).
            var cases = new System.Collections.Generic.List<object> { null };
            if (!type.IsAbstract) cases.Add(Activator.CreateInstance(type));
            foreach (Type candidate in type.Assembly.GetTypes())
                if (!candidate.IsAbstract && candidate != type && type.IsAssignableFrom(candidate)
                    && candidate.GetConstructor(Type.EmptyTypes) != null)
                    cases.Add(Activator.CreateInstance(candidate));
            return cases.ToArray();
        }

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered external-call method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered external-call signature differs: " + original.Name);
            var cases = new System.Collections.Generic.List<object[]>();
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            else
                foreach (object first in Candidates(parameters[0]))
                {
                    if (parameters.Length == 1) cases.Add(new[] { first });
                    else foreach (object second in Candidates(parameters[1]))
                        cases.Add(new[] { first, second });
                }
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                string expected = before.Error != null ? "throw:" + before.Error.Name : ExternalCallDescriptor(before.Value);
                string actual = after.Error != null ? "throw:" + after.Error.Name : ExternalCallDescriptor(after.Value);
                if (expected != actual)
                    throw new Exception("Recovered external call differs: " + original.Name
                        + " expected=" + expected + " actual=" + actual
                        + " args=" + string.Join("|", arguments.Select(a => a == null ? "null" : a.GetType().Name)));
                checks++;
            }
        }
        Console.WriteLine($"External call behavior: {checks} comparisons, {selected.Length} recovered methods; dispatch through a derived instance, null receivers, argument order and results match.");
    }
}
