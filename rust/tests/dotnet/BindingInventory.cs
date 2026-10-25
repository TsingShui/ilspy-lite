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
using System.Collections.Immutable;
using System.IO;
using System.Linq;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Text.Json;

// Reads definition and reference rows without loading the input assembly.
internal static class BindingInventory
{
    static int Token(EntityHandle handle) => handle.IsNil ? 0 : MetadataTokens.GetToken(handle);

    internal sealed class SignatureNames(MetadataReader md, string assembly) : ISignatureTypeProvider<string, object>
    {
        internal string DefinitionName(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parent = type.GetDeclaringType();
            string name = md.GetString(type.Name);
            string ns = md.GetString(type.Namespace);
            return !parent.IsNil ? DefinitionName(parent) + "/" + name : ns.Length == 0 ? name : ns + "." + name;
        }

        internal string ReferenceName(TypeReferenceHandle handle)
        {
            var type = md.GetTypeReference(handle);
            string name = md.GetString(type.Name);
            if (type.ResolutionScope.Kind == HandleKind.TypeReference)
                return ReferenceName((TypeReferenceHandle)type.ResolutionScope) + "/" + name;
            string ns = md.GetString(type.Namespace);
            string scope = type.ResolutionScope.Kind == HandleKind.AssemblyReference
                ? md.GetString(md.GetAssemblyReference((AssemblyReferenceHandle)type.ResolutionScope).Name) : assembly;
            return scope + "::" + (ns.Length == 0 ? name : ns + "." + name);
        }

        public string GetTypeFromDefinition(MetadataReader reader, TypeDefinitionHandle handle, byte rawTypeKind)
            => (rawTypeKind == 0x11 ? "value:" : "class:") + assembly + "::" + DefinitionName(handle);
        public string GetTypeFromReference(MetadataReader reader, TypeReferenceHandle handle, byte rawTypeKind)
            => (rawTypeKind == 0x11 ? "value:" : "class:") + ReferenceName(handle);
        public string GetTypeFromSpecification(MetadataReader reader, object context, TypeSpecificationHandle handle, byte rawTypeKind)
            => reader.GetTypeSpecification(handle).DecodeSignature(this, context);
        public string GetGenericInstantiation(string genericType, ImmutableArray<string> arguments)
            => genericType + "<" + string.Join(",", arguments) + ">";
        public string GetGenericMethodParameter(object context, int index) => "mvar:" + index;
        public string GetGenericTypeParameter(object context, int index) => "var:" + index;
        public string GetArrayType(string elementType, ArrayShape shape)
            => elementType + "[" + shape.Rank + ";" + string.Join(",", shape.Sizes) + ";" + string.Join(",", shape.LowerBounds) + "]";
        public string GetByReferenceType(string elementType) => "byref:" + elementType;
        public string GetPointerType(string elementType) => "ptr:" + elementType;
        public string GetSZArrayType(string elementType) => "szarray:" + elementType;
        public string GetPinnedType(string elementType) => "pinned:" + elementType;
        public string GetModifiedType(string modifier, string type, bool required) => "mod:" + required + ":" + modifier + ":" + type;
        public string GetFunctionPointerType(MethodSignature<string> signature) => JsonSerializer.Serialize(Shape(signature));
        public string GetPrimitiveType(PrimitiveTypeCode code) => code switch {
            PrimitiveTypeCode.Int32 => "I4", PrimitiveTypeCode.UInt32 => "U4",
            PrimitiveTypeCode.Int64 => "I8", PrimitiveTypeCode.UInt64 => "U8",
            PrimitiveTypeCode.Int16 => "I2", PrimitiveTypeCode.UInt16 => "U2",
            PrimitiveTypeCode.SByte => "I1", PrimitiveTypeCode.Byte => "U1",
            PrimitiveTypeCode.Single => "R4", PrimitiveTypeCode.Double => "R8",
            _ => code.ToString()
        };
    }

    static object Shape(MethodSignature<string> signature) => new {
        header = signature.Header.RawValue, generic_count = signature.GenericParameterCount,
        required_count = signature.RequiredParameterCount, result = signature.ReturnType,
        parameters = signature.ParameterTypes.ToArray()
    };

    static void Main(string[] args)
    {
        if (args.Length != 1) throw new ArgumentException("Expected one assembly path");
        using var pe = new PEReader(File.OpenRead(args[0]));
        var md = pe.GetMetadataReader();
        var definition = md.GetAssemblyDefinition();
        string assembly = md.GetString(definition.Name);
        var names = new SignatureNames(md, assembly);
        string Blob(BlobHandle handle) => Convert.ToHexString(md.GetBlobBytes(handle));
        object Identity(StringHandle name, Version version, StringHandle culture, BlobHandle key, int flags) => new {
            name = md.GetString(name), version = new[] { version.Major, version.Minor, version.Build, version.Revision },
            culture = md.GetString(culture), key = Blob(key), flags
        };
        object[] Generics(GenericParameterHandleCollection handles) => handles.Select(h => {
            var row = md.GetGenericParameter(h);
            return (object)new { token = Token(h), index = row.Index, name = md.GetString(row.Name), flags = (int)row.Attributes };
        }).ToArray();
        byte[] metadata = pe.GetMetadata().GetContent().ToArray();
        int RowOffset(TableIndex table, int rid) => md.GetTableMetadataOffset(table) + (rid - 1) * md.GetTableRowSize(table);
        string Row(TableIndex table, int rid) => Convert.ToHexString(metadata.AsSpan(RowOffset(table, rid), md.GetTableRowSize(table)));
        int FileOffset(TableIndex table, int rid) => pe.PEHeaders.MetadataStartOffset + RowOffset(table, rid);
        int StringOffset(StringHandle handle) => pe.PEHeaders.MetadataStartOffset + md.GetHeapMetadataOffset(HeapIndex.String) + MetadataTokens.GetHeapOffset(handle);
        var types = md.TypeDefinitions.Select(h => {
            var row = md.GetTypeDefinition(h);
            return new {
                token = Token(h), name = names.DefinitionName(h), flags = (int)row.Attributes,
                parent = Token(row.GetDeclaringType()), base_type = Token(row.BaseType),
                interfaces = row.GetInterfaceImplementations().Select(i => Token(md.GetInterfaceImplementation(i).Interface)).ToArray(),
                generics = Generics(row.GetGenericParameters()),
                methods = row.GetMethods().Select(m => {
                    var method = md.GetMethodDefinition(m);
                    return new { token = Token(m), owner = Token(h), name = md.GetString(method.Name), flags = (int)method.Attributes,
                        signature = Blob(method.Signature), shape = Shape(method.DecodeSignature(names, null)),
                        generics = Generics(method.GetGenericParameters()), name_index = MetadataTokens.GetHeapOffset(method.Name),
                        name_offset = StringOffset(method.Name) };
                }).ToArray(),
                fields = row.GetFields().Select(f => {
                    var field = md.GetFieldDefinition(f);
                    return new { token = Token(f), owner = Token(h), name = md.GetString(field.Name), flags = (int)field.Attributes,
                        signature = Blob(field.Signature), shape = field.DecodeSignature(names, null) };
                }).ToArray(),
                properties = row.GetProperties().Select(p => {
                    var property = md.GetPropertyDefinition(p);
                    var accessors = property.GetAccessors();
                    return new { token = Token(p), owner = Token(h), name = md.GetString(property.Name), flags = (int)property.Attributes,
                        signature = Blob(property.Signature), shape = Shape(property.DecodeSignature(names, null)),
                        getter = Token(accessors.Getter), setter = Token(accessors.Setter), others = accessors.Others.Select(m => Token(m)).ToArray() };
                }).ToArray()
            };
        }).ToArray();
        var references = md.TypeReferences.Select(h => {
            var row = md.GetTypeReference(h);
            return new { token = Token(h), name = md.GetString(row.Name), ns = md.GetString(row.Namespace),
                scope = Token(row.ResolutionScope), full_name = names.ReferenceName(h), raw_row = Row(TableIndex.TypeRef, MetadataTokens.GetRowNumber(h)) };
        }).ToArray();
        var specs = Enumerable.Range(1, md.GetTableRowCount(TableIndex.TypeSpec)).Select(rid => {
            var row = md.GetTypeSpecification(MetadataTokens.TypeSpecificationHandle(rid));
            var reader = md.GetBlobReader(row.Signature);
            if (reader.ReadSignatureTypeCode() != SignatureTypeCode.GenericTypeInstance)
                throw new BadImageFormatException("BindingCases TypeSpec must be a generic instantiation");
            var kind = reader.ReadSignatureTypeCode();
            if (kind != SignatureTypeCode.TypeHandle) throw new BadImageFormatException("Expected named TypeSpec definition");
            var target = reader.ReadTypeHandle();
            int count = reader.ReadCompressedInteger();
            var decoder = new SignatureDecoder<string, object>(names, md, null);
            var arguments = Enumerable.Range(0, count).Select(_ => decoder.DecodeType(ref reader)).ToArray();
            if (reader.RemainingBytes != 0) throw new BadImageFormatException("Trailing TypeSpec bytes");
            return new { token = 0x1b000000 | rid, definition = Token(target), arguments, signature = Blob(row.Signature),
                shape = row.DecodeSignature(names, null), raw_row = Row(TableIndex.TypeSpec, rid) };
        }).ToArray();
        var members = md.MemberReferences.Select(h => {
            var row = md.GetMemberReference(h);
            bool field = row.GetKind() == MemberReferenceKind.Field;
            return new { token = Token(h), owner = Token(row.Parent), name = md.GetString(row.Name), kind = field ? "field" : "method",
                signature = Blob(row.Signature), shape = field ? (object)row.DecodeFieldSignature(names, null) : Shape(row.DecodeMethodSignature(names, null)),
                raw_row = Row(TableIndex.MemberRef, MetadataTokens.GetRowNumber(h)) };
        }).ToArray();
        var methodSpecs = Enumerable.Range(1, md.GetTableRowCount(TableIndex.MethodSpec)).Select(rid => {
            var row = md.GetMethodSpecification(MetadataTokens.MethodSpecificationHandle(rid));
            return new { token = 0x2b000000 | rid, method = Token(row.Method), signature = Blob(row.Signature),
                arguments = row.DecodeSignature(names, null).ToArray(), raw_row = Row(TableIndex.MethodSpec, rid) };
        }).ToArray();
        var exports = md.ExportedTypes.Select(h => {
            var row = md.GetExportedType(h);
            return new { token = Token(h), name = md.GetString(row.Name), ns = md.GetString(row.Namespace),
                flags = (int)row.Attributes, implementation = Token(row.Implementation), raw_row = Row(TableIndex.ExportedType, MetadataTokens.GetRowNumber(h)) };
        }).ToArray();
        var semantics = Enumerable.Range(1, md.GetTableRowCount(TableIndex.MethodSemantics)).Select(rid => {
            int offset = RowOffset(TableIndex.MethodSemantics, rid);
            int methodWidth = md.GetTableRowCount(TableIndex.MethodDef) >= 65536 ? 4 : 2;
            int associationWidth = md.GetTableRowSize(TableIndex.MethodSemantics) - 2 - methodWidth;
            int method = methodWidth == 4 ? BitConverter.ToInt32(metadata, offset + 2) : BitConverter.ToUInt16(metadata, offset + 2);
            int association = associationWidth == 4 ? BitConverter.ToInt32(metadata, offset + 2 + methodWidth) : BitConverter.ToUInt16(metadata, offset + 2 + methodWidth);
            return new { token = 0x18000000 | rid, flags = BitConverter.ToUInt16(metadata, offset), method = 0x06000000 | method,
                association = ((association & 1) == 1 ? 0x17000000 : 0x14000000) | (association >> 1), raw_row = Row(TableIndex.MethodSemantics, rid) };
        }).ToArray();
        var referenceMarkers = definition.GetCustomAttributes().Select(h => (Handle: h, Row: md.GetCustomAttribute(h)))
            .Where(a => a.Row.Constructor.Kind == HandleKind.MemberReference)
            .Select(a => (a.Handle, a.Row, Constructor: md.GetMemberReference((MemberReferenceHandle)a.Row.Constructor)))
            .Where(a => a.Constructor.Parent.Kind == HandleKind.TypeReference
                && names.ReferenceName((TypeReferenceHandle)a.Constructor.Parent) == "System.Runtime::System.Runtime.CompilerServices.ReferenceAssemblyAttribute")
            .Select(a => new { token = Token(a.Handle), constructor = Token(a.Row.Constructor), name = md.GetString(a.Constructor.Name),
                signature = Blob(a.Constructor.Signature), value = Blob(a.Row.Value) }).ToArray();
        var assemblyRefs = md.AssemblyReferences.Select(h => {
            var row = md.GetAssemblyReference(h);
            // AssemblyRef ends in Name, Culture and HashValue heap indices.
            int blobWidth = md.GetHeapSize(HeapIndex.Blob) > 65535 ? 4 : 2;
            int stringWidth = (md.GetTableRowSize(TableIndex.AssemblyRef) - 12 - 2 * blobWidth) / 2;
            return new { token = Token(h), identity = Identity(row.Name, row.Version, row.Culture, row.PublicKeyOrToken, (int)row.Flags),
                raw_row = Row(TableIndex.AssemblyRef, MetadataTokens.GetRowNumber(h)), name_width = stringWidth,
                name_index_offset = FileOffset(TableIndex.AssemblyRef, MetadataTokens.GetRowNumber(h)) + 12 + blobWidth };
        }).ToArray();
        Console.WriteLine(JsonSerializer.Serialize(new {
            schema = 1, oracle = "PEReader/MetadataReader; static metadata only",
            identity = Identity(definition.Name, definition.Version, definition.Culture, definition.PublicKey, (int)definition.Flags),
            mvid = md.GetGuid(md.GetModuleDefinition().Mvid), types, type_refs = references, type_specs = specs,
            member_refs = members, method_specs = methodSpecs, exported_types = exports, assembly_refs = assemblyRefs,
            method_semantics = semantics, reference_markers = referenceMarkers
        }, new JsonSerializerOptions { WriteIndented = true }));
    }
}
