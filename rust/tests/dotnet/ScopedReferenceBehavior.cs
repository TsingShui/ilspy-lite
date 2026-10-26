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
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    record Specialization(string Carrier, int Token, MethodInfo Original, Type[] Types, Type[] Methods);
    static Type[] BindingTypes => new[] { typeof(int), typeof(string), typeof(decimal), typeof(object),
        typeof(double), fixture.GetType("ScopedReferenceCases.Payload") ?? throw new Exception("Owned payload missing."),
        typeof(DayOfWeek), typeof(int?), typeof(float), typeof(nint) };

    static List<Specialization> Specializations()
    {
        MethodInfo[] methods = fixture.GetTypes().SelectMany(t => t.GetMethods(BindingFlags.Public | BindingFlags.Static
            | BindingFlags.Instance | BindingFlags.DeclaredOnly)).Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal) || m.Name == "UnsupportedBare" || m.Name == "UnsupportedCall" || m.Name == "UnsupportedBox")
            .OrderBy(m => m.MetadataToken).ToArray();
        if (methods.Length != 25) throw new Exception("Scoped owned method selection drift.");
        var result = new List<Specialization>();
        foreach (MethodInfo definition in methods)
        {
            Type owner = definition.DeclaringType;
            int typeCount = owner.GetGenericArguments().Length;
            int methodCount = definition.GetGenericArguments().Length;
            if (typeCount + methodCount == 0) throw new Exception("Owned scoped method has no generic context.");
            for (int seed = 0; seed < BindingTypes.Length; seed++)
            {
                Type[] types = Enumerable.Range(0, typeCount).Select(i => BindingTypes[(seed + i) % BindingTypes.Length]).ToArray();
                Type[] arguments = Enumerable.Range(0, methodCount).Select(i => BindingTypes[(seed + typeCount + i) % BindingTypes.Length]).ToArray();
                Type closedOwner = typeCount == 0 ? owner : owner.MakeGenericType(types);
                MethodInfo closed = closedOwner.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance
                    | BindingFlags.DeclaredOnly).Single(m => m.MetadataToken == definition.MetadataToken);
                if (methodCount != 0) closed = closed.MakeGenericMethod(arguments);
                if (closed.ContainsGenericParameters) throw new Exception("Incomplete owned generic closure.");
                result.Add(new Specialization("S" + result.Count.ToString("D4"), definition.MetadataToken, closed, types, arguments));
            }
        }
        return result;
    }

    static void ScopedPlan(string directivesPath, List<Specialization> cases)
    {
        using var json = JsonDocument.Parse(File.ReadAllText(directivesPath));
        var directives = json.RootElement.EnumerateArray().ToDictionary(row => row.GetProperty("token").GetInt32());
        if (directives.Count != 25) throw new Exception("Scoped signature directive coverage drift.");
        var result = new List<object>();
        foreach (Specialization test in cases)
        {
            MethodInfo original = test.Original;
            var aliases = new List<string>();
            foreach (var entry in directives[test.Token].GetProperty("aliases").EnumerateArray())
            {
                string alias = entry.GetProperty("alias").GetString();
                if (!Regex.IsMatch(alias, "^__cil_(ref|generic)_[0-9a-f]{40}$")) throw new Exception("Invalid scoped alias.");
                int typeOwner = entry.GetProperty("type_owner").GetInt32();
                int methodOwner = entry.GetProperty("method_owner").GetInt32();
                if ((typeOwner != 0 && typeOwner != original.DeclaringType.MetadataToken)
                    || (methodOwner != 0 && methodOwner != original.MetadataToken))
                    throw new Exception("Generic alias owner differs from original declaration.");
                byte[] bytes = Convert.FromHexString(entry.GetProperty("bytes").GetString());
                int position = 0;
                Type type = Decode(bytes, ref position, typeOwner == 0 ? null : test.Types,
                    methodOwner == 0 ? null : test.Methods);
                bool variable = bytes[0] == 0x13 || bytes[0] == 0x1e;
                if (position != bytes.Length || type.ContainsGenericParameters
                    || variable != alias.StartsWith("__cil_generic_", StringComparison.Ordinal)
                    || !variable && type.IsValueType)
                    throw new Exception("Invalid specialized reference alias.");
                aliases.Add("using " + alias + " = " + TypeSource(type) + ";");
            }
            result.Add(new { carrier = test.Carrier, token = test.Token, name = original.Name, aliases });
        }
        Console.WriteLine(JsonSerializer.Serialize(result));
    }

    static object Sample(Type type, int variant, int depth = 0)
    {
        if (depth > 6) throw new Exception("Owned sample nesting exceeded.");
        if (type == typeof(int)) return variant == 0 ? int.MinValue : int.MaxValue;
        if (type == typeof(string)) return variant == 0 ? "first" : "second";
        if (type == typeof(decimal)) return variant == 0 ? new decimal(100, 0, 0, false, 2) : decimal.MinValue;
        if (type == typeof(double)) return variant == 0 ? -0.0 : BitConverter.Int64BitsToDouble(0x7ff8000000000123);
        if (type == typeof(float)) return variant == 0 ? -0.0f : BitConverter.Int32BitsToSingle(0x7fc00123);
        if (type == typeof(nint)) return variant == 0 ? nint.MinValue : nint.MaxValue;
        if (type.IsEnum) return Enum.ToObject(type, variant == 0 ? -1 : 7);
        if (Nullable.GetUnderlyingType(type) is Type nullable) return Sample(nullable, variant, depth + 1);
        if (type == typeof(object)) return new object();
        if (type.IsArray)
        {
            Type element = type.GetElementType();
            Array array = Array.CreateInstance(element, 2);
            array.SetValue(Sample(element, variant, depth + 1), 0);
            array.SetValue(Sample(element, 1 - variant, depth + 1), 1);
            return array;
        }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(List<>))
        {
            var list = (IList)Activator.CreateInstance(type);
            Type element = type.GetGenericArguments()[0];
            list.Add(Sample(element, variant, depth + 1));
            list.Add(Sample(element, 1 - variant, depth + 1));
            return list;
        }
        if (type.IsGenericType && type.GetGenericTypeDefinition() == typeof(Dictionary<,>))
        {
            var dictionary = (IDictionary)Activator.CreateInstance(type);
            Type[] arguments = type.GetGenericArguments();
            dictionary.Add(Sample(arguments[0], variant, depth + 1), Sample(arguments[1], variant, depth + 1));
            return dictionary;
        }
        if (type.Assembly == fixture)
        {
            object value = Activator.CreateInstance(type);
            foreach (FieldInfo member in type.GetFields(BindingFlags.Instance | BindingFlags.Public))
                member.SetValue(value, Sample(member.FieldType, variant, depth + 1));
            return value;
        }
        throw new Exception("Unsupported owned sample type: " + type);
    }

    static void ScopedCheck(Specialization test, Assembly generatedAssembly)
    {
        MethodInfo original = test.Original;
        MethodInfo generated = generatedAssembly.GetType(test.Carrier + ".Recovered")?.GetMethod(original.Name,
            BindingFlags.Public | BindingFlags.Static) ?? throw new Exception("Specialized recovered method missing.");
        Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
        Type[] expected = original.IsStatic ? parameters : parameters.Prepend(original.DeclaringType).ToArray();
        if (generated.ReturnType != original.ReturnType
            || !generated.GetParameters().Select(p => p.ParameterType).SequenceEqual(expected))
            throw new Exception("Scoped runtime parameter/return identity differs: " + original.Name);

        void Compare(object receiver, object[] arguments, object expectedValue)
        {
            var graph = CaptureGraph(arguments.Prepend(receiver));
            void Restore() { foreach (State slot in graph) slot.Write(slot.Initial); }
            Outcome before = Capture(() => OriginalCall(original)(receiver, arguments), graph);
            Restore();
            Outcome after = Capture(() => generated.Invoke(null, original.IsStatic ? arguments : arguments.Prepend(receiver).ToArray()), graph);
            Restore();
            bool boxedValue = original.Name == "UnsupportedBox" && parameters[0].IsValueType;
            bool SameResult(object left, object right) => original.ReturnType.IsValueType || boxedValue ? Same(left, right) : ReferenceEquals(left, right);
            if (boxedValue && after.Value != null && ReferenceEquals(after.Value, arguments[0]))
                throw new Exception("Recovered scoped boxing reused argument identity.");
            if (before.Error != null || !SameResult(before.Value, expectedValue))
                throw new Exception("Owned scoped expectation differs: " + original.Name);
            if (before.Error != after.Error || !SameResult(before.Value, after.Value)
                || !before.State.Zip(after.State).All(pair => pair.First.Length == pair.Second.Length
                    && pair.First.Zip(pair.Second).All(values => Same(values.First, values.Second)))
                || !before.Enumeration.Zip(after.Enumeration).All(values => Same(values.First, values.Second)))
                throw new Exception("Recovered scoped behavior differs: " + original.Name);
            checks++;
        }

        string name = original.Name;
        if (name == "Null") { Compare(null, Array.Empty<object>(), null); }
        else if (name == "Unused")
        {
            foreach (int value in new[] { int.MinValue, -1, 0, 1, int.MaxValue }) Compare(null, new object[] { value }, value);
        }
        else
        {
            Type valueType = parameters.First(p => p != typeof(bool));
            object first = Sample(valueType, 0), second = Sample(valueType, 1);
            object[] values = { valueType.IsValueType ? Activator.CreateInstance(valueType) : null, first, second };
            if (name == "Choose" || name == "MixedChoose" || name == "ChooseValue")
                foreach (object left in values)
                    foreach (object right in values)
                    {
                        Compare(null, new object[] { true, left, right }, left);
                        Compare(null, new object[] { false, left, right }, right);
                    }
            else if (name == "AssignArgument" || name == "AssignValue" || name == "SnapshotValue")
                foreach (object left in values)
                    foreach (object right in values) Compare(null, new[] { left, right }, name == "SnapshotValue" ? left : right);
            else if (name == "MixedValue")
                foreach (object value in values) Compare(null, new[] { value, Sample(parameters[1], 1) }, value);
            else
                foreach (object value in values)
                {
                    Compare(original.IsStatic ? null : Activator.CreateInstance(original.DeclaringType), new[] { value }, value);
                    if (!original.IsStatic) Compare(null, new[] { value }, value);
                }
        }
    }

    static void ScopedMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("ScopedReferenceCases.MethodOps") == null)
            throw new Exception("Expected owned scoped reference fixture and plan/check target.");
        var cases = Specializations();
        if (args[1] == "--scoped-plan") ScopedPlan(args[2], cases);
        else
        {
            Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
            foreach (Specialization test in cases) ScopedCheck(test, generated);
            Console.WriteLine($"Scoped value/reference behavior: {checks} comparisons, {cases.Count} closed contexts, 25 recovered methods; exact runtime signatures, value bits, reference identity, state and exceptions match.");
        }
    }
}
