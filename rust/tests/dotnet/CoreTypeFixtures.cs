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
using System.IO;
using System.Reflection;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;

// Author owned metadata-only test inputs. These images are never runtime-loaded.
internal static class CoreTypeFixtures
{
    static byte[] Build(string name, bool marker, bool wrongKey = false, string culture = "", bool genericMarker = false)
    {
        var md = new MetadataBuilder();
        var il = new BlobBuilder();
        il.WriteInt32(0);
        byte[] key = new byte[16];
        key[8] = wrongKey ? (byte)5 : (byte)4;
        md.AddModule(0, md.GetOrAddString("OwnedCore.dll"),
            md.GetOrAddGuid(new Guid("c0e845d5-bef0-4000-8000-0123456789ab")), default, default);
        var assembly = md.AddAssembly(md.GetOrAddString(name), new Version(4, 0, 0, 0),
            md.GetOrAddString(culture), md.GetOrAddBlob(key), AssemblyFlags.PublicKey, AssemblyHashAlgorithm.Sha1);
        var firstField = MetadataTokens.FieldDefinitionHandle(1);
        var firstMethod = MetadataTokens.MethodDefinitionHandle(1);
        TypeDefinitionHandle Type(string ns, string typeName, EntityHandle parent, int field, int method,
            TypeAttributes flags = TypeAttributes.Public) => md.AddTypeDefinition(flags,
                md.GetOrAddString(ns), md.GetOrAddString(typeName), parent,
                MetadataTokens.FieldDefinitionHandle(field), MetadataTokens.MethodDefinitionHandle(method));
        md.AddTypeDefinition(0, default, md.GetOrAddString("<Module>"), default, firstField, firstMethod);
        var root = Type("System", "Object", default, 1, 1);
        var value = Type("System", "ValueType", root, 1, 3, TypeAttributes.Public | TypeAttributes.Abstract);
        var enumeration = Type("System", "Enum", value, 1, 3, TypeAttributes.Public | TypeAttributes.Abstract);
        Type("Example", "Number", value, 1, 3, TypeAttributes.Public | TypeAttributes.Sealed);
        Type("Example", "Flags", enumeration, 2, 4, TypeAttributes.Public | TypeAttributes.Sealed);
        Type("Example", "Subject", root, 3, 5);
        Type("Other", "Object", default, 4, 7);
        Type("Example", "Cyclic", MetadataTokens.TypeDefinitionHandle(9), 4, 8);
        var generic = Type("Example", "GenericRoot", default, 4, 9);
        var attribute = Type("System", "Attribute", root, 4, 10, TypeAttributes.Public | TypeAttributes.Abstract);
        Type("System.Runtime.CompilerServices", "ReferenceAssemblyAttribute", attribute, 4, 11,
            TypeAttributes.Public | TypeAttributes.Sealed);
        Type("System", "String", root, 4, 12, TypeAttributes.Public | TypeAttributes.Sealed);
        var exception = Type("System", "Exception", root, 4, 13);
        Type("System", "OwnedException", exception, 4, 14);
        Type("Example", "Unrelated", root, 4, 15);
        Type("Example", "ExceptionProbe", root, 4, 16);
        md.AddGenericParameter(generic, 0, md.GetOrAddString("T"), 0);
        md.AddFieldDefinition(FieldAttributes.Public, md.GetOrAddString("Value"), md.GetOrAddBlob(new byte[] { 6, 8 }));
        md.AddFieldDefinition(FieldAttributes.Public | FieldAttributes.SpecialName | FieldAttributes.RTSpecialName,
            md.GetOrAddString("value__"), md.GetOrAddBlob(new byte[] { 6, 8 }));
        md.AddFieldDefinition(FieldAttributes.Public, md.GetOrAddString("Value"), md.GetOrAddBlob(new byte[] { 6, 8 }));
        int Body(params byte[] code)
        {
            il.Align(4);
            int offset = il.Count;
            il.WriteByte((byte)((code.Length << 2) | 2));
            il.WriteBytes(code);
            return offset;
        }
        MethodDefinitionHandle Method(string methodName, bool constructor, params byte[] code) =>
            md.AddMethodDefinition(MethodAttributes.Public | (constructor ? MethodAttributes.SpecialName | MethodAttributes.RTSpecialName : 0),
                MethodImplAttributes.IL, md.GetOrAddString(methodName),
                md.GetOrAddBlob(new byte[] { 0x20, 0, constructor ? (byte)1 : (byte)8 }), Body(code), MetadataTokens.ParameterHandle(1));
        Method("RootConstant", false, 0x1f, 42, 0x2a);
        Method(".ctor", true, 0x2a);
        Method("ReadNumber", false, 0x02, 0x7b, 1, 0, 0, 4, 0x2a);
        Method("ReadEnum", false, 0x1f, 7, 0x2a);
        Method("Read", false, 0x02, 0x7b, 3, 0, 0, 4, 0x2a);
        Method(".ctor", true, 0x02, 0x28, 2, 0, 0, 6, 0x2a);
        Method("OtherConstant", false, 0x1f, 11, 0x2a);
        Method("CycleConstant", false, 0x1f, 12, 0x2a);
        Method("GenericConstant", false, 0x1f, 13, 0x2a);
        Method(".ctor", true, 0x02, 0x28, 2, 0, 0, 6, 0x2a);
        var markerCtor = Method(".ctor", true, 0x02, 0x28, 10, 0, 0, 6, 0x2a);
        md.AddMethodDefinition(MethodAttributes.Public, MethodImplAttributes.IL, md.GetOrAddString("ReturnSelf"),
            md.GetOrAddBlob(new byte[] { 0x20, 0, 0x0e }), Body(0x02, 0x2a), MetadataTokens.ParameterHandle(1));
        Method(".ctor", true, 0x02, 0x28, 2, 0, 0, 6, 0x2a);
        Method(".ctor", true, 0x02, 0x28, 13, 0, 0, 6, 0x2a);
        Method(".ctor", true, 0x02, 0x28, 2, 0, 0, 6, 0x2a);
        Method("ThrowLocal", false, 0x73, 14, 0, 0, 6, 0x7a);
        Method("ThrowUnrelated", false, 0x73, 15, 0, 0, 6, 0x7a);
        if (genericMarker) md.AddGenericParameter(markerCtor, 0, md.GetOrAddString("U"), 0);
        if (marker) md.AddCustomAttribute(assembly, markerCtor, md.GetOrAddBlob(new byte[] { 1, 0, 0, 0 }));
        var pe = new BlobBuilder();
        new ManagedPEBuilder(PEHeaderBuilder.CreateLibraryHeader(), new MetadataRootBuilder(md), il,
            deterministicIdProvider: _ => new BlobContentId(new Guid("c0e845d5-bef0-4000-8000-0123456789ab"), 0)).Serialize(pe);
        return pe.ToArray();
    }

    static void Main(string[] args)
    {
        if (args.Length != 1) throw new ArgumentException("Expected owned fixture output directory.");
        Directory.CreateDirectory(args[0]);
        void Write(string file, byte[] bytes) => File.WriteAllBytes(Path.Combine(args[0], file), bytes);
        Write("core-types.bin", Build("mscorlib", false));
        Write("core-reference.bin", Build("mscorlib", true));
        Write("core-fake-name.bin", Build("OwnedCore", false));
        Write("core-fake-marker.bin", Build("OwnedCore", true));
        Write("core-generic-marker.bin", Build("mscorlib", true, genericMarker: true));
        Write("core-wrong-key.bin", Build("mscorlib", false, wrongKey: true));
        Write("core-wrong-culture.bin", Build("mscorlib", false, culture: "en-US"));
    }
}
