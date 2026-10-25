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
using System.IO;
using System.Linq;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Text.Json;

// Independent static metadata oracle; does not load or execute the input assembly.
internal static class ClassInventory
{
    static string Token(EntityHandle handle) => handle.IsNil ? "0x00000000" : $"0x{MetadataTokens.GetToken(handle):X8}";

    static void Main(string[] args)
    {
        if (args.Length != 2) throw new ArgumentException("Expected assembly path and exact type name");
        using var pe = new PEReader(File.OpenRead(args[0]));
        MetadataReader md = pe.GetMetadataReader();
        string TypeName(TypeDefinitionHandle handle)
        {
            TypeDefinition type = md.GetTypeDefinition(handle);
            string name = md.GetString(type.Name);
            var parent = type.GetDeclaringType();
            if (!parent.IsNil) return TypeName(parent) + "/" + name;
            string ns = md.GetString(type.Namespace);
            return ns.Length == 0 ? name : ns + "." + name;
        }
        byte[] metadata = pe.GetMetadata().GetContent().ToArray();
        byte[] Row(TableIndex table, int rid)
        {
            int size = md.GetTableRowSize(table);
            int offset = md.GetTableMetadataOffset(table) + (rid - 1) * size;
            return metadata.AsSpan(offset, size).ToArray();
        }
        object[] Layouts(TypeDefinitionHandle owner) => Enumerable.Range(1, md.GetTableRowCount(TableIndex.ClassLayout))
            .Select(rid => (Rid: rid, Bytes: Row(TableIndex.ClassLayout, rid)))
            .Where(row => (row.Bytes.Length == 8 ? BitConverter.ToUInt16(row.Bytes, 6) : BitConverter.ToUInt32(row.Bytes, 6))
                == MetadataTokens.GetRowNumber(owner))
            .Select(row => (object)new { token = $"0x{0x0f000000 | row.Rid:X8}", parent = Token(owner),
                packing = BitConverter.ToUInt16(row.Bytes, 0), size = BitConverter.ToUInt32(row.Bytes, 2),
                raw_row = Convert.ToHexString(row.Bytes) }).ToArray();
        string Blob(BlobHandle handle) => Convert.ToHexString(md.GetBlobBytes(handle));
        object Constant(ConstantHandle handle)
        {
            if (handle.IsNil) return null;
            var constant = md.GetConstant(handle);
            return new { type = constant.TypeCode.ToString(), value = Blob(constant.Value) };
        }
        object[] Generics(GenericParameterHandleCollection handles) => handles.Select(handle => {
            var parameter = md.GetGenericParameter(handle);
            return (object)new { token = Token(handle), name = md.GetString(parameter.Name), index = parameter.Index,
                flags = (int)parameter.Attributes, constraints = parameter.GetConstraints().Select(c =>
                    Token(md.GetGenericParameterConstraint(c).Type)).ToArray() };
        }).ToArray();
        string MethodSignature(EntityHandle handle) => Blob(handle.Kind == HandleKind.MethodDefinition
            ? md.GetMethodDefinition((MethodDefinitionHandle)handle).Signature
            : md.GetMemberReference((MemberReferenceHandle)handle).Signature);
        string MethodName(EntityHandle handle) => md.GetString(handle.Kind == HandleKind.MethodDefinition
            ? md.GetMethodDefinition((MethodDefinitionHandle)handle).Name
            : md.GetMemberReference((MemberReferenceHandle)handle).Name);
        string MethodOwner(EntityHandle handle) => Token(handle.Kind == HandleKind.MethodDefinition
            ? md.GetMethodDefinition((MethodDefinitionHandle)handle).GetDeclaringType()
            : md.GetMemberReference((MemberReferenceHandle)handle).Parent);
        object[] Implementations(TypeDefinitionHandle handle) => md.GetTypeDefinition(handle).GetMethodImplementations().Select(h => {
            var record = md.GetMethodImplementation(h);
            return (object)new { token = Token(h), @class = Token(record.Type), body = Token(record.MethodBody),
                declaration = Token(record.MethodDeclaration), body_signature = MethodSignature(record.MethodBody),
                declaration_signature = MethodSignature(record.MethodDeclaration),
                body_name = MethodName(record.MethodBody), body_owner = MethodOwner(record.MethodBody),
                declaration_name = MethodName(record.MethodDeclaration), declaration_owner = MethodOwner(record.MethodDeclaration),
                raw_row = Convert.ToHexString(Row(TableIndex.MethodImpl, MetadataTokens.GetRowNumber(h))) };
        }).ToArray();
        object[] Security(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parents = new HashSet<EntityHandle> { handle };
            foreach (var method in type.GetMethods()) parents.Add(method);
            if (md.IsAssembly) parents.Add(MetadataTokens.EntityHandle(0x20000001));
            return md.DeclarativeSecurityAttributes.Where(h => parents.Contains(md.GetDeclarativeSecurityAttribute(h).Parent)).Select(h => {
                var record = md.GetDeclarativeSecurityAttribute(h);
                return (object)new { token = Token(h), parent = Token(record.Parent), action = (int)record.Action,
                    permission_set_blob_index = MetadataTokens.GetHeapOffset(record.PermissionSet), permission_set = Blob(record.PermissionSet),
                    raw_row = Convert.ToHexString(Row(TableIndex.DeclSecurity, MetadataTokens.GetRowNumber(h))) };
            }).ToArray();
        }
        object[] Attributes(TypeDefinitionHandle handle)
        {
            var type = md.GetTypeDefinition(handle);
            var parents = new HashSet<EntityHandle> { handle, MetadataTokens.EntityHandle(0x00000001) };
            if (md.IsAssembly) parents.Add(MetadataTokens.EntityHandle(0x20000001));
            void AddGenerics(GenericParameterHandleCollection generics)
            {
                foreach (var generic in generics)
                {
                    parents.Add(generic);
                    foreach (var constraint in md.GetGenericParameter(generic).GetConstraints()) parents.Add(constraint);
                }
            }
            AddGenerics(type.GetGenericParameters());
            foreach (var field in type.GetFields()) parents.Add(field);
            foreach (var property in type.GetProperties()) parents.Add(property);
            foreach (var evt in type.GetEvents()) parents.Add(evt);
            foreach (var relation in type.GetInterfaceImplementations()) parents.Add(relation);
            foreach (var method in type.GetMethods())
            {
                parents.Add(method);
                var definition = md.GetMethodDefinition(method);
                foreach (var parameter in definition.GetParameters()) parents.Add(parameter);
                foreach (var security in definition.GetDeclarativeSecurityAttributes()) parents.Add(security);
                AddGenerics(definition.GetGenericParameters());
            }
            foreach (var security in type.GetDeclarativeSecurityAttributes()) parents.Add(security);
            if (md.IsAssembly)
                foreach (var security in md.GetAssemblyDefinition().GetDeclarativeSecurityAttributes()) parents.Add(security);
            return md.CustomAttributes.Where(h => parents.Contains(md.GetCustomAttribute(h).Parent)).Select(h => {
                var attribute = md.GetCustomAttribute(h);
                BlobHandle signature = attribute.Constructor.Kind == HandleKind.MethodDefinition
                    ? md.GetMethodDefinition((MethodDefinitionHandle)attribute.Constructor).Signature
                    : md.GetMemberReference((MemberReferenceHandle)attribute.Constructor).Signature;
                return (object)new { token = Token(h), parent = Token(attribute.Parent), constructor = Token(attribute.Constructor),
                    value_blob_index = MetadataTokens.GetHeapOffset(attribute.Value), value = Blob(attribute.Value),
                    constructor_signature = Blob(signature), constructor_name = MethodName(attribute.Constructor), constructor_owner = MethodOwner(attribute.Constructor),
                    raw_row = Convert.ToHexString(Row(TableIndex.CustomAttribute, MetadataTokens.GetRowNumber(h))) };
            }).ToArray();
        }
        var root = md.TypeDefinitions.Single(h => TypeName(h) == args[1]);
        bool Owned(TypeDefinitionHandle handle)
        {
            if (handle == root) return true;
            var parent = md.GetTypeDefinition(handle).GetDeclaringType();
            return !parent.IsNil && Owned(parent);
        }
        var types = md.TypeDefinitions.Where(Owned).Select(handle => {
            var type = md.GetTypeDefinition(handle);
            return new {
                token = Token(handle), name = TypeName(handle), flags = (int)type.Attributes,
                parent = Token(type.GetDeclaringType()), base_type = Token(type.BaseType),
                interfaces = type.GetInterfaceImplementations().Select(i => Token(md.GetInterfaceImplementation(i).Interface)).ToArray(),
                class_layout = Layouts(handle),
                custom_attributes = Attributes(handle),
                security_declarations = Security(handle),
                method_implementations = Implementations(handle),
                generic_parameters = Generics(type.GetGenericParameters()),
                fields = type.GetFields().Select(h => {
                    var field = md.GetFieldDefinition(h);
                    return new { token = Token(h), name = md.GetString(field.Name), flags = (int)field.Attributes,
                        signature = Blob(field.Signature), constant = Constant(field.GetDefaultValue()) };
                }).ToArray(),
                methods = type.GetMethods().Select(h => {
                    var method = md.GetMethodDefinition(h);
                    return new { token = Token(h), name = md.GetString(method.Name), flags = (int)method.Attributes,
                        impl_flags = (int)method.ImplAttributes, signature = Blob(method.Signature),
                        has_rva = method.RelativeVirtualAddress != 0,
                        generic_parameters = Generics(method.GetGenericParameters()),
                        parameters = method.GetParameters().Select(p => {
                            var parameter = md.GetParameter(p);
                            return new { token = Token(p), name = md.GetString(parameter.Name), sequence = parameter.SequenceNumber,
                                flags = (int)parameter.Attributes, constant = Constant(parameter.GetDefaultValue()) };
                        }).ToArray() };
                }).ToArray(),
                properties = type.GetProperties().Select(h => {
                    var property = md.GetPropertyDefinition(h);
                    var a = property.GetAccessors();
                    return new { token = Token(h), name = md.GetString(property.Name), flags = (int)property.Attributes,
                        signature = Blob(property.Signature), getter = Token(a.Getter), setter = Token(a.Setter),
                        others = a.Others.Select(x => Token(x)).ToArray(), constant = Constant(property.GetDefaultValue()) };
                }).ToArray(),
                events = type.GetEvents().Select(h => {
                    var evt = md.GetEventDefinition(h);
                    var a = evt.GetAccessors();
                    return new { token = Token(h), name = md.GetString(evt.Name), flags = (int)evt.Attributes,
                        event_type = Token(evt.Type), adder = Token(a.Adder), remover = Token(a.Remover), raiser = Token(a.Raiser),
                        others = a.Others.Select(x => Token(x)).ToArray() };
                }).ToArray()
            };
        }).ToArray();
        Console.WriteLine(JsonSerializer.Serialize(new {
            schema = 1, oracle = "System.Reflection.Metadata (static PEReader; no input execution)",
            mvid = md.GetGuid(md.GetModuleDefinition().Mvid), root = Token(root), types
        }, new JsonSerializerOptions { WriteIndented = true }));
    }
}
