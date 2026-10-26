using System;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    static object DeclaringInstance(Type type, int count, int value)
    {
        object created = Activator.CreateInstance(type, count);
        type.GetField("Value").SetValue(created, value);
        return created;
    }

    static string DeclaringDescriptor(object value, object[] arguments)
    {
        if (value == null) return "null";
        for (int index = 0; index < arguments.Length; index++)
            if (ReferenceEquals(arguments[index], value)) return "arg" + index;
        Type type = value.GetType();
        if (type.IsPrimitive || value is string)
            return Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);
        return type.Name + "{" + string.Join(",", type
            .GetFields(BindingFlags.Public | BindingFlags.Instance)
            .OrderBy(f => f.Name)
            .Select(f => f.Name + "=" + DeclaringDescriptor(f.GetValue(value), Array.Empty<object>()))) + "}";
    }

    static void GenericDeclaringMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("GenericDeclaringCases.Ops") == null)
            throw new Exception("Expected owned generic-declaring fixture.");
        Type ops = fixture.GetType("GenericDeclaringCases.Ops");
        Type holder = fixture.GetType("GenericDeclaringCases.Holder`1").MakeGenericType(typeof(int));
        Type slot = fixture.GetType("GenericDeclaringCases.Slot`1").MakeGenericType(typeof(int));
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("GenericDeclaringCases.Recovered")
            ?? throw new Exception("Recovered generic-declaring carrier missing.");
        MethodInfo[] selected = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly)
            .Select(lowered => ops.GetMethod(lowered.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + lowered.Name))
            .OrderBy(m => m.MetadataToken).ToArray();
        if (selected.Length == 0) throw new Exception("Owned generic-declaring selection is empty.");

        object holderA = DeclaringInstance(holder, 3, 9);
        object holderB = DeclaringInstance(holder, 7, 11);
        object slotValue = Activator.CreateInstance(slot);
        slot.GetField("Content").SetValue(slotValue, 5);

        object[] Candidates(Type type)
        {
            if (type == holder) return new[] { holderA, holderB };
            if (type == slot) return new[] { slotValue };
            if (type == typeof(int)) return new object[] { 0, 4, -2 };
            if (type.IsValueType) return new[] { Activator.CreateInstance(type) };
            return new object[] { null };
        }

        foreach (MethodInfo original in selected)
        {
            MethodInfo lowered = carrier.GetMethod(original.Name, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("Missing recovered generic-declaring method: " + original.Name);
            Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
            if (lowered.ReturnType != original.ReturnType
                || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                throw new Exception("Recovered generic-declaring signature differs: " + original.Name);
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
                string expected = before.Error != null ? "throw:" + before.Error.Name : DeclaringDescriptor(before.Value, arguments);
                string actual = after.Error != null ? "throw:" + after.Error.Name : DeclaringDescriptor(after.Value, arguments);
                if (expected != actual)
                    throw new Exception("Recovered generic-declaring member differs: " + original.Name
                        + " expected=" + expected + " actual=" + actual);
                checks++;
            }
        }
        Console.WriteLine($"Generic declaring behavior: {checks} comparisons, {selected.Length} recovered methods; open self receivers, substituting field types and identity results match.");
    }
}
