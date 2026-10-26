using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.Loader;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    // A type initializer is described by what the platform reports when it fails
    // and by the static state it leaves behind. Nothing else about it is
    // observable, and nothing about it may be repeated.
    static string StaticInitializationFailure(Type type)
    {
        try
        {
            RuntimeHelpers.RunClassConstructor(type.TypeHandle);
            return "none";
        }
        catch (Exception exception)
        {
            return StaticFailure(exception);
        }
    }

    static string StaticFailure(Exception exception)
    {
        string inner = exception.InnerException == null ? "" : "<" + StaticFailure(exception.InnerException) + ">";
        // The platform's own wrapper message names the initialized type, which
        // differs by construction; the argument detail is what the body chose.
        string detail = exception is ArgumentException argument
            ? "[" + argument.ParamName + "|" + argument.Message + "]"
            : "";
        return exception.GetType().Name + inner + detail;
    }

    static bool SameStaticState(object left, object right)
    {
        if (left is string text && right is string other) return text == other;
        return Same(left, right);
    }

    static string StaticState(object value) => value == null ? "null"
        : value is string text ? "\"" + text + "\""
        : Convert.ToString(value, System.Globalization.CultureInfo.InvariantCulture);

    static void StaticConstructorMain(string[] args)
    {
        if (args.Length != 4 || fixture.GetType("StaticConstructorCases.Simple") == null)
            throw new Exception("Expected owned static-constructor fixture and its recovered carrier.");
        Assembly carrier = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
        string source = File.ReadAllText(args[3]);
        // The carrier may only use shells whose token names a real fixture member of
        // the expected kind, so a placeholder can never stand for a different one.
        foreach (Match match in Regex.Matches(source, @"__cil_ldstr_([0-9A-F]{8})\("))
        {
            int token = Convert.ToInt32(match.Groups[1].Value, 16);
            if ((uint)token >> 24 != 0x70) throw new Exception("Invalid string literal token in carrier.");
            fixture.ManifestModule.ResolveString(token);
        }
        foreach (Match match in Regex.Matches(source, @"__cil_newobj_([0-9A-F]{8})\("))
        {
            int token = Convert.ToInt32(match.Groups[1].Value, 16);
            MethodBase target = fixture.ManifestModule.ResolveMethod(token);
            if (target is not ConstructorInfo constructor
                || constructor.DeclaringType != typeof(ArgumentOutOfRangeException)
                || constructor.IsStatic
                || !constructor.GetParameters().Select(p => p.ParameterType)
                    .SequenceEqual(new[] { typeof(string), typeof(string) }))
                throw new Exception("Static constructor exception allocation target mismatch.");
        }
        var recovered = carrier.GetTypes()
            .Where(t => t.Name.StartsWith("Recovered", StringComparison.Ordinal) && !t.Name.EndsWith("Shells", StringComparison.Ordinal))
            .OrderBy(t => t.Name, StringComparer.Ordinal).ToArray();
        if (recovered.Length == 0) throw new Exception("Recovered static-constructor carrier is empty.");
        var pairs = recovered.SelectMany(type => {
            Type original = fixture.GetType("StaticConstructorCases." + type.Name.Substring("Recovered".Length))
                ?? throw new Exception("Recovered carrier has no owned counterpart: " + type.Name);
            if (!type.IsGenericTypeDefinition) return new[] { (Original: original, Recovered: type) };
            if (type.GetGenericArguments().Length != 1 || original.GetGenericArguments().Length != 1)
                throw new Exception("Unexpected owned initializer arity.");
            return new[] { typeof(int), typeof(string), typeof(object) }.Select(argument =>
                (Original: original.MakeGenericType(argument), Recovered: type.MakeGenericType(argument)));
        }).ToArray();
        // Every original initializer runs first, then every recovered one, in the
        // same order, so a body that reads another type's static state reads the
        // same value in both phases.
        var expected = pairs.Select(pair => StaticInitializationFailure(pair.Original)).ToArray();
        var actual = pairs.Select(pair => StaticInitializationFailure(pair.Recovered)).ToArray();
        for (int index = 0; index < pairs.Length; index++)
        {
            string name = pairs[index].Original.ToString();
            if (expected[index] != actual[index])
                throw new Exception("Recovered static constructor differs: " + name
                    + " expected=" + expected[index] + " actual=" + actual[index]);
            if (expected[index] != "none")
            {
                // A failed initialization poisons the type: the platform refuses
                // every later field read, so no state is observable on either side.
                checks++;
                Covered.Add(name);
                continue;
            }
            FieldInfo[] fields = pairs[index].Original.GetFields(BindingFlags.Public | BindingFlags.Static);
            if (fields.Length == 0) throw new Exception("Owned static constructor has no observable state: " + name);
            foreach (FieldInfo original in fields)
            {
                FieldInfo lowered = pairs[index].Recovered.GetField(original.Name, BindingFlags.Public | BindingFlags.Static)
                    ?? throw new Exception("Recovered type lost field: " + name + "." + original.Name);
                if (lowered.FieldType != original.FieldType || lowered.IsInitOnly != original.IsInitOnly)
                    throw new Exception("Recovered field type differs: " + name + "." + original.Name);
                object before = original.GetValue(null);
                object after = lowered.GetValue(null);
                if (!SameStaticState(before, after))
                    throw new Exception("Recovered static constructor state differs: " + name + "." + original.Name
                        + " expected=" + StaticState(before) + " actual=" + StaticState(after));
                checks++;
            }
            Covered.Add(name);
        }
        Console.WriteLine($"Static constructor behavior: {checks} state comparisons over {Covered.Count} recovered types; "
            + "initialization order, one-time execution, static state and failure mode match.");
    }
}
