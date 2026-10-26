using System;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    // A result is described by its value, not by identity: the point of this
    // milestone is that the call reaches the same implementation, not that the
    // same instance comes back.
    static object ExternalValueSample(Type type, ref int counter)
    {
        object created = Activator.CreateInstance(type);
        foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name))
            field.SetValue(created, field.FieldType == typeof(byte) ? (byte)(0xE0 + counter++)
                : field.FieldType.IsPrimitive ? ++counter
                : ExternalValueSample(field.FieldType, ref counter));
        return created;
    }

    static string ExternalValueDescriptor(object value)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        if (type.IsPrimitive || value is string)
            return "value:" + Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        FieldInfo[] fields = type.GetFields(BindingFlags.Public | BindingFlags.Instance);
        if (fields.Length == 0) return "type:" + type.Name;
        return type.Name + "{" + string.Join(",", fields.OrderBy(f => f.Name)
            .Select(f => f.Name + "=" + ExternalValueDescriptor(f.GetValue(value)))) + "}";
    }

    static void ExternalValueMain(string[] args)
    {
        if (args.Length != 4 || fixture.GetType("ExternalValueCases.Ops") == null)
            throw new Exception("Expected owned external-call fixture and its library.");
        string libraryPath = System.IO.Path.GetFullPath(args[3]);
        Assembly resolved = null;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
        {
            if (name.Name == "ExternalValueLibrary") return resolved ??= context.LoadFromAssemblyPath(libraryPath);
            return null;
        };
        resolved = AssemblyLoadContext.Default.LoadFromAssemblyPath(libraryPath);
        Type ops = fixture.GetType("ExternalValueCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExternalValueCases.Recovered")
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
            // A value type is described by its field contents, so the sample fills
            // every public field with a distinguishable value.
            int counter = 0;
            return new object[] { ExternalValueSample(type, ref counter) };
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
                string expected = before.Error != null ? "throw:" + before.Error.Name : ExternalValueDescriptor(before.Value);
                string actual = after.Error != null ? "throw:" + after.Error.Name : ExternalValueDescriptor(after.Value);
                if (expected != actual)
                    throw new Exception("Recovered external call differs: " + original.Name
                        + " expected=" + expected + " actual=" + actual
                        + " args=" + string.Join("|", arguments.Select(a => a == null ? "null" : a.GetType().Name)));
                checks++;
            }
        }
        Console.WriteLine($"External value behavior: {checks} comparisons, {selected.Length} recovered methods; whole-value copies of externally defined structs and enums preserve field contents.");
    }
}
