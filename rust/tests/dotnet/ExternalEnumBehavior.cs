using System;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    // Enums cross the reflection boundary as boxed values, so an enum result is
    // described by its underlying integer rather than by the boxed instance.
    static string ExternalEnumDescriptor(object value)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        if (type.IsEnum) return "enum:" + Convert.ToInt64(value).ToString();
        // A primitive result is only compared if its value is part of the
        // description; a widened load that changed the number must surface here.
        if (type.IsPrimitive)
            return "value:" + Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        if (type.IsValueType) return "struct:" + type.FullName + "{" + string.Join(",", type.GetFields(BindingFlags.Public | BindingFlags.Instance)
            .OrderBy(field => field.Name).Select(field => field.Name + "=" + ExternalEnumDescriptor(field.GetValue(value)))) + "}";
        return "identity:" + type.Name + ":" + System.Runtime.CompilerServices.RuntimeHelpers.GetHashCode(value)
            + "{" + string.Join(",", type.GetFields(BindingFlags.Public | BindingFlags.Instance)
                .OrderBy(field => field.Name).Select(field => field.Name + "=" + ExternalEnumDescriptor(field.GetValue(value)))) + "}";
    }

    static void ExternalEnumMain(string[] args)
    {
        if (args.Length != 4 || fixture.GetType("ExternalEnumCases.Ops") == null)
            throw new Exception("Expected owned external-enum fixture and its library.");
        string libraryPath = System.IO.Path.GetFullPath(args[3]);
        Assembly resolved = null;
        AssemblyLoadContext.Default.Resolving += (context, name) =>
        {
            if (name.Name == "ExternalEnumLibrary") return resolved ??= context.LoadFromAssemblyPath(libraryPath);
            return null;
        };
        resolved = AssemblyLoadContext.Default.LoadFromAssemblyPath(libraryPath);
        Type ops = fixture.GetType("ExternalEnumCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ExternalEnumCases.Recovered")
            ?? throw new Exception("Recovered external-enum carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned external-enum selection is empty.");

        object[] Candidates(Type type)
        {
            if (type.IsEnum)
            {
                long[] values = { 0, 1, 2, unchecked((long)0xFFFFFFFFFFFFFFFF) };
                return values.Select(v => Enum.ToObject(type, v)).ToArray();
            }
            if (type == typeof(object) || type == typeof(string)) return new object[] { null };
            object WithFields(int value)
            {
                object result = Activator.CreateInstance(type);
                foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance))
                {
                    if (field.FieldType != typeof(int)) throw new Exception("Unexpected owned field: " + field);
                    field.SetValue(result, value);
                }
                return result;
            }
            if (type.IsValueType) return new object[] { WithFields(0), WithFields(-7), WithFields(int.MaxValue) };
            return new object[] { null, WithFields(17) };
        }

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered external-enum method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered external-enum signature differs: " + original.Name);
            var cases = new System.Collections.Generic.List<object[]>();
            if (parameters.Length == 0) cases.Add(Array.Empty<object>());
            else
                foreach (object first in Candidates(parameters[0]))
                    cases.Add(new[] { first });
            foreach (object[] arguments in cases)
            {
                Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
                string expected = before.Error != null ? "throw:" + before.Error.Name : ExternalEnumDescriptor(before.Value);
                string beforeState = string.Join("|", arguments.Select(ExternalEnumDescriptor));
                Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
                string actual = after.Error != null ? "throw:" + after.Error.Name : ExternalEnumDescriptor(after.Value);
                if (expected != actual || beforeState != string.Join("|", arguments.Select(ExternalEnumDescriptor)))
                    throw new Exception("Recovered external-enum differs: " + original.Name + " expected=" + expected + " actual=" + actual + " args=" + string.Join("|", arguments.Select(a => a == null ? "null" : a.GetType().Name + ":" + Convert.ToString(a, System.Globalization.CultureInfo.InvariantCulture))));
                checks++;
            }
        }
        Console.WriteLine($"External enum behavior: {checks} comparisons, {selected.Length} recovered methods; declared-reference storage widths, loads, stores and results match.");
    }
}
