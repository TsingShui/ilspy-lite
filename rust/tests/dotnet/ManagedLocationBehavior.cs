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
using System.Reflection.Emit;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Runtime.Loader;

internal static partial class GenericReferenceRoundtrip
{
    public class ManagedArgument<T> { public T Value = default; }

    static Func<object[], object> ManagedInvoker(MethodInfo method)
    {
        var body = new DynamicMethod("Managed_" + method.Name, typeof(object), new[] { typeof(object[]) },
            typeof(GenericReferenceRoundtrip).Module, true);
        var il = body.GetILGenerator();
        foreach (ParameterInfo parameter in method.GetParameters())
        {
            il.Emit(OpCodes.Ldarg_0); il.Emit(OpCodes.Ldc_I4, parameter.Position); il.Emit(OpCodes.Ldelem_Ref);
            Type type = parameter.ParameterType;
            if (type.IsByRef)
            {
                Type cell = typeof(ManagedArgument<>).MakeGenericType(type.GetElementType());
                il.Emit(OpCodes.Castclass, cell); il.Emit(OpCodes.Ldflda, cell.GetField("Value"));
            }
            else il.Emit(type.IsValueType ? OpCodes.Unbox_Any : OpCodes.Castclass, type);
        }
        il.Emit(OpCodes.Call, method);
        if (method.ReturnType == typeof(void)) il.Emit(OpCodes.Ldnull);
        else if (method.ReturnType.IsValueType) il.Emit(OpCodes.Box, method.ReturnType);
        il.Emit(OpCodes.Ret);
        return (Func<object[], object>)body.CreateDelegate(typeof(Func<object[], object>));
    }

    static void ManagedInitOnlyVariant(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned initonly variant path.");
        byte[] image = System.IO.File.ReadAllBytes(args[0]);
        using var pe = new PEReader(new System.IO.MemoryStream(image));
        var metadata = pe.GetMetadataReader();
        if (args[1] == "--managed-bool-variant")
        {
            MethodInfo flag = fixture.GetType("ManagedLocationCases.Ops").GetMethod("Flag");
            var definition = metadata.GetMethodDefinition(MetadataTokens.MethodDefinitionHandle(flag.MetadataToken & 0xffffff));
            int rva = definition.RelativeVirtualAddress;
            var section = pe.PEHeaders.SectionHeaders.Single(s => rva >= s.VirtualAddress && rva < s.VirtualAddress + s.VirtualSize);
            int start = section.PointerToRawData + rva - section.VirtualAddress;
            int header = (image[start] & 3) == 2 ? 1
                : (System.Buffers.Binary.BinaryPrimitives.ReadUInt16LittleEndian(image.AsSpan(start, 2)) >> 12) * 4;
            byte[] code = flag.GetMethodBody().GetILAsByteArray();
            int instruction = Enumerable.Range(0, code.Length - 2).Single(i => code[i] == 2 && code[i+1] == 0x17 && code[i+2] == 0x52);
            if (!image.AsSpan(start + header, code.Length).SequenceEqual(code)) throw new Exception("Owned bool IL offset differs.");
            image[start + header + instruction + 1] = 0x18;
            MethodInfo bits = fixture.GetType("ManagedLocationCases.Ops").GetMethod("BooleanBits");
            var bitsDef = metadata.GetMethodDefinition(MetadataTokens.MethodDefinitionHandle(bits.MetadataToken & 0xffffff));
            int bitsRva = bitsDef.RelativeVirtualAddress;
            var bitsSection = pe.PEHeaders.SectionHeaders.Single(s => bitsRva >= s.VirtualAddress && bitsRva < s.VirtualAddress + s.VirtualSize);
            int bitsStart = bitsSection.PointerToRawData + bitsRva - bitsSection.VirtualAddress;
            int bitsHeader = (image[bitsStart] & 3) == 2 ? 1
                : (System.Buffers.Binary.BinaryPrimitives.ReadUInt16LittleEndian(image.AsSpan(bitsStart, 2)) >> 12) * 4;
            byte[] bitsCode = bits.GetMethodBody().GetILAsByteArray();
            if (bitsCode.Length < 3 || !image.AsSpan(bitsStart + bitsHeader, bitsCode.Length).SequenceEqual(bitsCode))
                throw new Exception("Owned bool load IL offset differs.");
            image.AsSpan(bitsStart + bitsHeader, bitsCode.Length).Clear();
            image[bitsStart + bitsHeader] = 2;
            image[bitsStart + bitsHeader + 1] = (byte)OpCodes.Ldind_U1.Value;
            image[bitsStart + bitsHeader + bitsCode.Length - 1] = (byte)OpCodes.Ret.Value;
            System.IO.File.WriteAllBytes(args[2], image);
            return;
        }
        FieldInfo number = fixture.GetType("ManagedLocationCases.Cell")?.GetField("Number")
            ?? throw new Exception("Owned Number field missing.");
        int rid = number.MetadataToken & 0xffffff;
        if (number.IsInitOnly || number.IsStatic || number.FieldType != typeof(int))
            throw new Exception("Unexpected owned field flags/type.");
        int offset = pe.PEHeaders.MetadataStartOffset + metadata.GetTableMetadataOffset(TableIndex.Field)
            + (rid - 1) * metadata.GetTableRowSize(TableIndex.Field);
        ushort flags = System.Buffers.Binary.BinaryPrimitives.ReadUInt16LittleEndian(image.AsSpan(offset, 2));
        if (flags != (ushort)number.Attributes) throw new Exception("Owned field table offset differs.");
        System.Buffers.Binary.BinaryPrimitives.WriteUInt16LittleEndian(image.AsSpan(offset, 2), (ushort)(flags | 0x20));
        System.IO.File.WriteAllBytes(args[2], image);
    }

    static void ManagedFieldAddressPlan(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned field-address token plan.");
        using var input = System.Text.Json.JsonDocument.Parse(System.IO.File.ReadAllText(args[2]));
        foreach (var item in input.RootElement.EnumerateArray())
        {
            string opcode = item.GetProperty("opcode").GetString();
            string hex = item.GetProperty("token").GetString();
            int token = int.Parse(hex, System.Globalization.NumberStyles.HexNumber);
            FieldInfo field = fixture.ManifestModule.ResolveField(token);
            bool isStatic = opcode == "ldsflda";
            if ((opcode != "ldsflda" && opcode != "ldflda") || field.IsStatic != isStatic
                || !field.IsInitOnly || field.DeclaringType.ContainsGenericParameters || field.FieldType.ContainsGenericParameters)
                throw new Exception("Unproved owned field address.");
            string value = TypeSource(field.FieldType), owner = TypeSource(field.DeclaringType);
            string delegateType = isStatic ? "__CilStaticField<" + value + ">"
                : (field.DeclaringType.IsValueType ? "__CilValueField<" : "__CilReferenceField<") + owner + ", " + value + ">";
            string receiver = isStatic ? "" : (field.DeclaringType.IsValueType ? "ref " : "") + owner + " receiver";
            string argument = isStatic ? "" : (field.DeclaringType.IsValueType ? "ref " : "") + "receiver";
            Console.WriteLine("private static readonly System.Lazy<" + delegateType + "> field_" + hex
                + " = new(() => __CilBindField<" + delegateType + ">(" + token + "));");
            Console.WriteLine("private static ref " + value + " __cil_" + opcode + "_" + hex + "(" + receiver
                + ") => ref field_" + hex + ".Value(" + argument + ");");
        }
        Console.WriteLine(@"
private delegate ref V __CilStaticField<V>();
private delegate ref V __CilReferenceField<O,V>(O receiver);
private delegate ref V __CilValueField<O,V>(ref O receiver);
private static D __CilBindField<D>(int token) where D : System.Delegate
{
    var field = typeof(global::ManagedLocationCases.Ops).Module.ResolveField(token);
    var invoke = typeof(D).GetMethod(""Invoke"");
    var parameters = invoke.GetParameters().Select(p => p.ParameterType).ToArray();
    if (invoke.ReturnType != field.FieldType.MakeByRefType()
        || (field.IsStatic ? parameters.Length != 0 : parameters.Length != 1
            || parameters[0] != (field.DeclaringType.IsValueType ? field.DeclaringType.MakeByRefType() : field.DeclaringType)))
        throw new System.Exception(""Owned field leaf signature differs."");
    var method = new System.Reflection.Emit.DynamicMethod(""FieldAddress"", invoke.ReturnType, parameters,
        typeof(global::ManagedLocationCases.Ops).Module, true);
    var il = method.GetILGenerator();
    if (!field.IsStatic) il.Emit(System.Reflection.Emit.OpCodes.Ldarg_0);
    il.Emit(field.IsStatic ? System.Reflection.Emit.OpCodes.Ldsflda : System.Reflection.Emit.OpCodes.Ldflda, field);
    il.Emit(System.Reflection.Emit.OpCodes.Ret);
    return (D)method.CreateDelegate(typeof(D));
}
");
    }

    static void ManagedLocationMain(string[] args)
    {
        if (args.Length != 3) throw new Exception("Expected owned managed-location fixture and carrier.");
        Type ops = fixture.GetType("ManagedLocationCases.Ops") ?? throw new Exception("Owned Ops missing.");
        Type holder = fixture.GetType("ManagedLocationCases.Cell") ?? throw new Exception("Owned Cell missing.");
        Type payload = fixture.GetType("ManagedLocationCases.Payload") ?? throw new Exception("Owned Payload missing.");
        Type enumeration = fixture.GetType("ManagedLocationCases.Kind") ?? throw new Exception("Owned enum missing.");
        Assembly generated = AssemblyLoadContext.Default.LoadFromAssemblyPath(System.IO.Path.GetFullPath(args[2]));
        Type carrier = generated.GetType("ManagedLocationCases.Recovered") ?? throw new Exception("Owned carrier missing.");
        object shared = new object();
        string firstText = new string(new[] { 'f', 'i', 'r', 's', 't' });
        string nextText = new string(new[] { 'n', 'e', 'x', 't' });
        object Sample(Type type, int variant)
        {
            if (type == typeof(int)) return variant == 0 ? 0 : variant == 1 ? int.MinValue : 7;
            if (type == typeof(sbyte)) return variant == 0 ? (sbyte)0 : variant == 1 ? sbyte.MinValue : sbyte.MaxValue;
            if (type == typeof(char)) return variant == 0 ? (char)0 : variant == 1 ? char.MaxValue : (char)7;
            if (type == typeof(nint)) return variant == 0 ? (nint)0 : variant == 1 ? nint.MinValue : nint.MaxValue;
            if (type == typeof(nuint)) return variant == 0 ? (nuint)0 : variant == 1 ? nuint.MaxValue : (nuint)7;
            if (type == typeof(float)) return variant == 0 ? -0.0f : variant == 1 ? BitConverter.Int32BitsToSingle(0x7fc00031) : float.PositiveInfinity;
            if (type == typeof(byte)) return variant == 0 ? (byte)0 : variant == 1 ? byte.MaxValue : (byte)7;
            if (type == typeof(ushort)) return variant == 0 ? (ushort)0 : variant == 1 ? ushort.MaxValue : (ushort)7;
            if (type == typeof(ulong)) return variant == 0 ? 0UL : variant == 1 ? ulong.MaxValue : 7UL;
            if (type == typeof(bool))
            {
                byte bits = (byte)variant;
                return System.Runtime.CompilerServices.Unsafe.As<byte, bool>(ref bits);
            }
            if (type == typeof(string)) return variant == 0 ? null : variant == 1 ? firstText : nextText;
            if (type == typeof(object)) return variant == 0 ? null : shared;
            if (type == enumeration) return Enum.ToObject(type, variant == 0 ? 0 : variant == 1 ? short.MinValue : short.MaxValue);
            if (type == payload)
            {
                object value = Activator.CreateInstance(type);
                type.GetField("Number").SetValue(value, variant == 0 ? 0 : variant == 1 ? int.MinValue : int.MaxValue);
                type.GetField("Reference").SetValue(value, variant == 0 ? null : shared);
                return value;
            }
            if (type.FullName == "ManagedLocationCases.ReadOnlyCell") return variant == 0 ? null : Activator.CreateInstance(type);
            if (type.FullName == "ManagedLocationCases.Envelope") return Activator.CreateInstance(type, Sample(payload, variant));
            if (type == holder)
            {
                if (variant == 0) return null;
                object value = Activator.CreateInstance(holder);
                holder.GetField("Number").SetValue(value, variant == 1 ? 41 : -13);
                holder.GetField("Text").SetValue(value, variant == 1 ? firstText : nextText);
                holder.GetField("Data").SetValue(value, Sample(payload, variant));
                return value;
            }
            if (type == typeof(int[])) return variant == 0 ? null : variant == 1 ? Array.Empty<int>() : new[] { 17, -9 };
            throw new Exception("Unexpected managed-location scalar: " + type);
        }
        object[] Candidates(Type type, string name)
        {
            if (type.IsByRef)
            {
                Type element = type.GetElementType();
                Type box = typeof(ManagedArgument<>).MakeGenericType(element);
                return Enumerable.Range(0, 3).Select(index =>
                {
                    object value = Activator.CreateInstance(box);
                    box.GetField("Value").SetValue(value, Sample(element, index));
                    return value;
                }).ToArray();
            }
            if (type == typeof(int) && (name == "index" || name == "divisor"))
                return name == "index" ? new object[] { -1, 0, 1, 2 } : new object[] { 0, 2, -2 };
            return Enumerable.Range(0, 3).Select(index => Sample(type, index)).ToArray();
        }
        object[] Copy(object[] roots)
        {
            var copied = new Dictionary<object, object>(ReferenceEqualityComparer.Instance);
            object Clone(object value)
            {
                if (value == null || value is string || value.GetType().IsValueType || ReferenceEquals(value, shared)) return value;
                if (copied.TryGetValue(value, out object existing)) return existing;
                if (value is Array array) return array.Clone();
                object result = Activator.CreateInstance(value.GetType());
                copied.Add(value, result);
                foreach (FieldInfo field in value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance))
                    field.SetValue(result, Clone(field.GetValue(value)));
                return result;
            }
            return roots.Select(Clone).ToArray();
        }
        string Describe(object value, object[] roots)
        {
            if (value == null) return "null";
            if (ReferenceEquals(value, shared)) return "shared";
            if (value is string) return ReferenceEquals(value, firstText) ? "first-string"
                : ReferenceEquals(value, nextText) ? "next-string" : "new-string:" + value;
            if (value.GetType() == typeof(bool)) return "bool-bits:" + System.Runtime.CompilerServices.Unsafe.As<bool, byte>(ref System.Runtime.CompilerServices.Unsafe.Unbox<bool>(value));
            if (value is float single) return "single-bits:" + BitConverter.SingleToInt32Bits(single);
            if (value.GetType().IsPrimitive || value.GetType().IsEnum) return value.GetType().FullName + ":" + value;
            if (value is Array array) return "[" + string.Join(",", array.Cast<object>()) + "]";
            return value.GetType().Name + "{" + string.Join(",", value.GetType().GetFields(BindingFlags.Public | BindingFlags.Instance)
                .OrderBy(field => field.Name).Select(field => field.Name + "=" + Describe(field.GetValue(value), roots))) + "}";
        }
        MethodInfo[] definitions = carrier.GetMethods(BindingFlags.Public | BindingFlags.Static | BindingFlags.DeclaredOnly);
        if (definitions.Length == 0) throw new Exception("Owned managed-location selection empty.");
        int comparisons = 0, closures = 0;
        foreach (MethodInfo definition in definitions.OrderBy(m => m.Name))
        {
            MethodInfo originalDefinition = ops.GetMethod(definition.Name) ?? throw new Exception("Owned counterpart missing.");
            Type[][] instantiations = definition.IsGenericMethodDefinition
                ? new[] { new[] { typeof(int) }, new[] { typeof(string) }, new[] { typeof(object) }, new[] { payload }, new[] { enumeration }, new[] { typeof(float) }, new[] { typeof(bool) } }
                : new[] { Type.EmptyTypes };
            foreach (Type[] types in instantiations)
            {
                MethodInfo original = types.Length == 0 ? originalDefinition : originalDefinition.MakeGenericMethod(types);
                MethodInfo lowered = types.Length == 0 ? definition : definition.MakeGenericMethod(types);
                ParameterInfo[] parameters = original.GetParameters();
                if (original.ReturnType != lowered.ReturnType || !parameters.Select(p => p.ParameterType)
                    .SequenceEqual(lowered.GetParameters().Select(p => p.ParameterType)))
                    throw new Exception("Managed pointer element/ByRef signature differs: " + original.Name);
                var originalCall = ManagedInvoker(original);
                var recoveredCall = ManagedInvoker(lowered);
                var cases = new List<object[]> { Array.Empty<object>() };
                foreach (ParameterInfo parameter in parameters)
                    cases = cases.SelectMany(prefix => Candidates(parameter.ParameterType, parameter.Name)
                        .Select(value => prefix.Concat(new[] { value }).ToArray())).ToList();
                if (parameters.Length == 2 && parameters[0].ParameterType.IsByRef
                    && parameters[0].ParameterType == parameters[1].ParameterType)
                    foreach (object value in Candidates(parameters[0].ParameterType, parameters[0].Name))
                        cases.Add(new[] { value, value });
                if (original.Name == "UnsupportedJoin")
                    foreach (bool first in new[] { true, false })
                    foreach (object location in Candidates(parameters[1].ParameterType, parameters[1].Name))
                        cases.Add(new[] { (object)first, location, location });
                closures++;
                foreach (object[] arguments in cases)
                foreach (int globalVariant in new[] { 0, 1, 2 })
                {
                    object[] beforeRoots = Copy(arguments.Concat(new[] { Sample(holder, globalVariant) }).ToArray());
                    object[] afterRoots = Copy(arguments.Concat(new[] { Sample(holder, globalVariant) }).ToArray());
                    void Reset(Type owner, object[] roots)
                    {
                        owner.GetField("Trace").SetValue(null, 0);
                        owner.GetField("GlobalNumber").SetValue(null, 31);
                        owner.GetField("Global").SetValue(null, roots.Last());
                    }
                    Reset(ops, beforeRoots); Reset(carrier, afterRoots);
                    Outcome before = Capture(() => originalCall(beforeRoots.Take(arguments.Length).ToArray()), new List<State>());
                    Outcome after = Capture(() => recoveredCall(afterRoots.Take(arguments.Length).ToArray()), new List<State>());
                    string StateOf(Type owner, object[] roots) => string.Join("|", roots.Select(value => Describe(value, roots)))
                        + "|trace=" + owner.GetField("Trace").GetValue(null)
                        + "|number=" + owner.GetField("GlobalNumber").GetValue(null)
                        + "|global=" + Describe(owner.GetField("Global").GetValue(null), roots);
                    if (before.Error != after.Error || Describe(before.Value, beforeRoots) != Describe(after.Value, afterRoots)
                        || StateOf(ops, beforeRoots) != StateOf(carrier, afterRoots))
                        throw new Exception("Recovered managed location differs: " + original.Name
                            + " expected=" + (before.Error?.Name ?? Describe(before.Value, beforeRoots))
                            + " actual=" + (after.Error?.Name ?? Describe(after.Value, afterRoots))
                            + " state=" + StateOf(ops, beforeRoots) + "/" + StateOf(carrier, afterRoots));
                    comparisons++;
                }
            }
        }
        Console.WriteLine("Managed location behavior: " + comparisons + " comparisons, " + closures
            + " closed selections; aliased pointers, acquisition checks, original storage, generic/struct copies and exception writes match.");
    }
}
