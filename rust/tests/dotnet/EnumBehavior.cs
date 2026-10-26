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
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Runtime.Loader;
using System.Text.Json;
using System.Text.RegularExpressions;

internal static partial class GenericReferenceRoundtrip
{
    static (string Keyword, int Width, bool Signed) Storage(Type type) => Type.GetTypeCode(Enum.GetUnderlyingType(type)) switch {
        TypeCode.SByte => ("sbyte", 8, true), TypeCode.Byte => ("byte", 8, false),
        TypeCode.Int16 => ("short", 16, true), TypeCode.UInt16 => ("ushort", 16, false),
        TypeCode.Int32 => ("int", 32, true), TypeCode.UInt32 => ("uint", 32, false),
        TypeCode.Int64 => ("long", 64, true), TypeCode.UInt64 => ("ulong", 64, false),
        _ => throw new Exception("Unexpected enum storage.")
    };

    static ulong EnumBits(object value)
    {
        Type type = value.GetType();
        var storage = Storage(type);
        ulong bits = storage.Signed ? unchecked((ulong)Convert.ToInt64(value, CultureInfo.InvariantCulture))
            : Convert.ToUInt64(value, CultureInfo.InvariantCulture);
        return storage.Width == 64 ? bits : bits & ((1UL << storage.Width) - 1);
    }

    static void EnumMain(string[] args)
    {
        if (args.Length != 3 || fixture.GetType("EnumCases.S8Ops") == null) throw new Exception("Expected owned enum fixture.");
        Type[] enums = fixture.GetTypes().Where(t => t.IsEnum).OrderBy(t => t.MetadataToken).ToArray();
        if (enums.Length != 8) throw new Exception("Owned enum coverage drift.");
        if (args[1] == "--enum-plan")
        {
            using var json = JsonDocument.Parse(File.ReadAllText(args[2]));
            foreach (var item in json.RootElement.EnumerateArray())
            {
                int token = item.GetProperty("token").GetInt32();
                string alias = item.GetProperty("alias").GetString();
                if (!Regex.IsMatch(alias, "^__cil_enum_[0-9A-F]{8}$") || alias != "__cil_enum_" + token.ToString("X8"))
                    throw new Exception("Invalid enum identity alias.");
                Type type = fixture.ManifestModule.ResolveType(token);
                if (!type.IsEnum || type.Assembly != fixture || Storage(type).Keyword != item.GetProperty("storage").GetString())
                    throw new Exception("Enum storage differs from independent runtime metadata.");
                Console.WriteLine("using " + alias + " = " + TypeSource(type) + ";");
            }
            return;
        }
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(Path.GetFullPath(args[2]));
        foreach (Type type in enums)
        {
            Type owner = fixture.GetType("EnumCases." + type.Name + "Ops") ?? throw new Exception("Owned enum operations missing.");
            Type carrier = generated.GetType("Enum_" + type.Name + ".Recovered") ?? throw new Exception("Recovered enum carrier missing.");
            field = owner.GetField("Value") ?? throw new Exception("Owned enum field missing.");
            global = owner.GetField("Global") ?? throw new Exception("Owned enum static missing.");
            MethodInfo[] methods = owner.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.Instance
                | BindingFlags.DeclaredOnly).Where(m => !m.Name.StartsWith("Unsupported", StringComparison.Ordinal)).ToArray();
            if (methods.Length != 12) throw new Exception("Owned enum method selection drift.");
            var storage = Storage(type);
            ulong mask = storage.Width == 64 ? ulong.MaxValue : (1UL << storage.Width) - 1;
            ulong sign = 1UL << (storage.Width - 1);
            object[] values = new[] { 0UL, 1UL, mask, sign, sign - 1, (sign + 1) & mask,
                0xaaaaaaaaaaaaaaaaUL & mask, 0x5555555555555555UL & mask }.Distinct().Select(v => Enum.ToObject(type, v)).ToArray();
            object seed = Enum.ToObject(type, sign);
            global.SetValue(null, seed);

            void CheckEnum(string name, object receiver, object[] arguments, object expected, Type expectedError = null)
            {
                MethodInfo original = methods.Single(m => m.Name == name);
                MethodInfo lowered = carrier.GetMethod(name, BindingFlags.Public | BindingFlags.Static)
                    ?? throw new Exception("Missing recovered enum method.");
                Type[] parameters = original.GetParameters().Select(p => p.ParameterType).ToArray();
                if (!original.IsStatic) parameters = parameters.Prepend(owner).ToArray();
                if (lowered.ReturnType != original.ReturnType
                    || !lowered.GetParameters().Select(p => p.ParameterType).SequenceEqual(parameters))
                    throw new Exception("Enum carrier signature differs.");
                var graph = CaptureGraph(arguments.Prepend(receiver));
                void Restore() { foreach (State slot in graph) slot.Write(slot.Initial); }
                Outcome before = Capture(() => OriginalCall(original)(receiver, arguments), graph);
                Restore();
                Outcome after = Capture(() => lowered.Invoke(null, original.IsStatic ? arguments : arguments.Prepend(receiver).ToArray()), graph);
                Restore();
                if (before.Error != expectedError || !Same(before.Value, expected))
                    throw new Exception("Owned enum expectation differs: " + owner.Name + "." + name);
                if (before.Error != after.Error || !Same(before.Value, after.Value)
                    || !before.State.Zip(after.State).All(pair => pair.First.Length == pair.Second.Length
                        && pair.First.Zip(pair.Second).All(v => Same(v.First, v.Second))))
                    throw new Exception("Recovered enum behavior differs: " + owner.Name + "." + name);
                checks++;
            }

            if (args[1] == "--enum-forward") CheckEnum("Forward", null, new[] { seed }, seed);
            else
            {
                CheckEnum("Zero", null, Array.Empty<object>(), Enum.ToObject(type, 0));
                foreach (object value in values)
                {
                    CheckEnum("Identity", null, new[] { value }, value);
                    CheckEnum("Forward", null, new[] { value }, value);
                    CheckEnum("IsZero", null, new[] { value }, EnumBits(value) == 0);
                    long widened = storage.Signed ? Convert.ToInt64(value, CultureInfo.InvariantCulture) : unchecked((long)EnumBits(value));
                    CheckEnum("Widen", null, new[] { value }, widened);
                    object receiver = Activator.CreateInstance(owner);
                    field.SetValue(receiver, value);
                    CheckEnum("Read", receiver, Array.Empty<object>(), value);
                    CheckEnum("Exchange", receiver, new[] { seed }, value);
                    CheckEnum("StaticExchange", null, new[] { value }, seed);
                    foreach (object next in values)
                    {
                        CheckEnum("Snapshot", null, new[] { value, next }, value);
                        CheckEnum("AssignArgument", null, new[] { value, next }, next);
                        CheckEnum("Choose", null, new[] { (object)true, value, next }, value);
                        CheckEnum("Choose", null, new[] { (object)false, value, next }, next);
                        CheckEnum("SupportedOr", null, new[] { value, next }, Enum.ToObject(type, EnumBits(value) | EnumBits(next)));
                    }
                }
                CheckEnum("Read", null, Array.Empty<object>(), null, typeof(NullReferenceException));
                CheckEnum("Exchange", null, new[] { seed }, null, typeof(NullReferenceException));
            }

            // An unbox must be exercised with a real boxed value: the null case
            // alone never reaches the runtime type check or the storage conversion.
            MethodInfo unbox = owner.GetMethod("UnsupportedUnbox", BindingFlags.Public | BindingFlags.Static);
            MethodInfo loweredUnbox = carrier.GetMethod("UnsupportedUnbox", BindingFlags.Public | BindingFlags.Static);
            if (loweredUnbox != null && args[1] != "--enum-forward")
            {
                if (unbox == null)
                    throw new Exception("Recovered enum unbox has no owned counterpart: " + type.Name);
                if (loweredUnbox.ReturnType != unbox.ReturnType
                    || !loweredUnbox.GetParameters().Select(p => p.ParameterType).SequenceEqual(new[] { typeof(object) }))
                    throw new Exception("Recovered enum unbox signature differs: " + type.Name);
                void CheckUnbox(object argument, object expected, Type expectedError)
                {
                    var state = new System.Collections.Generic.List<State>();
                    Outcome before = Capture(() => OriginalCall(unbox)(null, new[] { argument }), state);
                    Outcome after = Capture(() => loweredUnbox.Invoke(null, new[] { argument }), state);
                    if (before.Error != expectedError || !Same(before.Value, expected))
                        throw new Exception("Owned enum unbox expectation differs: " + type.Name);
                    if (before.Error != after.Error || !Same(before.Value, after.Value))
                        throw new Exception("Recovered enum unbox differs: " + type.Name
                            + " expected=" + (before.Error?.Name ?? ExternalValueDescriptor(before.Value))
                            + " actual=" + (after.Error?.Name ?? ExternalValueDescriptor(after.Value)));
                    checks++;
                }
                foreach (object value in values) CheckUnbox(value, value, null);
                CheckUnbox(null, null, typeof(NullReferenceException));
                // A box of the storage integer unboxes into the enum (the CLI
                // accepts an enum's own storage type), while a sibling enum is a
                // different runtime type and must be rejected.
                CheckUnbox(Activator.CreateInstance(Enum.GetUnderlyingType(type)), Enum.ToObject(type, 0), null);
                Type sibling = enums.First(other => other != type);
                CheckUnbox(Enum.ToObject(sibling, 0), null, typeof(InvalidCastException));
            }
        }
        Console.WriteLine($"Enum behavior: {checks} comparisons; eight exact storage types, values/bits, snapshots, fields/statics, direct helper calls and exceptions match.");
    }
}
