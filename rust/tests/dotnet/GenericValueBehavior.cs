using System;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    // A constructed value has no identity to compare: what must match is the value
    // it carries, read through the fixture's own fields.
    static string CompositeDescriptor(object value)
    {
        if (value == null) return "null";
        Type type = value.GetType();
        if (type.IsPrimitive || value is string)
            return Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        if (type.IsEnum) return "enum:" + Convert.ToInt64(value).ToString();
        FieldInfo[] fields = type.GetFields(BindingFlags.Public | BindingFlags.Instance);
        if (fields.Length == 0) return "type:" + type.Name;
        return type.Name + "{" + string.Join(",", fields
            .OrderBy(f => f.Name)
            .Select(f => f.Name + "=" + CompositeDescriptor(f.GetValue(value)))) + "}";
    }

    static object Sample(Type type, ref int counter)
    {
        if (type == typeof(int)) return ++counter;
        if (type == typeof(long)) return (long)++counter;
        if (type == typeof(byte)) return (byte)(0xE0 + counter++);
        if (type == typeof(string)) return "s" + (++counter);
        object created = Activator.CreateInstance(type);
        foreach (FieldInfo field in type.GetFields(BindingFlags.Public | BindingFlags.Instance).OrderBy(f => f.Name))
            field.SetValue(created, Sample(field.FieldType, ref counter));
        return created;
    }

    static void GenericValueMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("GenericValueCases.Ops") == null)
            throw new Exception("Expected owned generic-value fixture.");
        Type ops = fixture.GetType("GenericValueCases.Ops");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("GenericValueCases.Recovered")
            ?? throw new Exception("Recovered generic-value carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned generic-value selection is empty.");

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered generic-value method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered generic-value signature differs: " + original.Name);
            var counter = 0;
            object[] arguments = parameters.Select(p => Sample(p, ref counter)).ToArray();
            Outcome before = Capture(() => OriginalCall(original)(null, arguments), new System.Collections.Generic.List<State>());
            Outcome after = Capture(() => lowered.Invoke(null, arguments), new System.Collections.Generic.List<State>());
            string expected = before.Error != null ? "throw:" + before.Error.Name : CompositeDescriptor(before.Value);
            string actual = after.Error != null ? "throw:" + after.Error.Name : CompositeDescriptor(after.Value);
            if (expected != actual)
                throw new Exception("Recovered generic value differs: " + original.Name
                    + " expected=" + expected + " actual=" + actual);
            checks++;
        }
        Console.WriteLine($"Generic value behavior: {checks} comparisons, {selected.Length} recovered methods; constructed value contents, argument copies and results match.");
    }
}
