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
using System.Reflection;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Security.Cryptography;
using System.Text.Json;

// Static acceptance oracle. Selected images are read as data, never runtime-loaded.
internal static class MetadataInventory
{
    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";

    static object Inspect(string path)
    {
        using var pe = new PEReader(File.OpenRead(path));
        if (!pe.HasMetadata)
        {
            if (pe.PEHeaders.CorHeader != null)
                throw new BadImageFormatException("CLR image has no readable metadata.");
            return new { path, kind = "native", complete = true, methods = Array.Empty<object>(), errors = Array.Empty<object>() };
        }
        var md = pe.GetMetadataReader(MetadataReaderOptions.None);
        string TypeName(TypeDefinitionHandle handle)
        {
            var parts = new List<string>();
            var seen = new HashSet<TypeDefinitionHandle>();
            string ns = "";
            while (!handle.IsNil)
            {
                if (parts.Count >= 64 || !seen.Add(handle))
                    throw new BadImageFormatException("Cyclic or excessive declaring type hierarchy.");
                var type = md.GetTypeDefinition(handle);
                parts.Add(md.GetString(type.Name));
                ns = md.GetString(type.Namespace);
                handle = type.GetDeclaringType();
            }
            parts.Reverse();
            return (ns.Length == 0 ? "" : ns + ".") + string.Join("/", parts);
        }
        bool CoreIdentity(StringHandle name, BlobHandle publicKey, StringHandle culture, AssemblyFlags flags)
        {
            byte[] key = md.GetBlobBytes(publicKey);
            if ((flags & AssemblyFlags.PublicKey) != 0)
            {
                byte[] hash = SHA1.HashData(key);
                key = hash.AsSpan(hash.Length - 8).ToArray();
                Array.Reverse(key);
            }
            string expected = md.GetString(name) switch {
                "System.Runtime" => "B03F5F7F11D50A3A",
                "mscorlib" => "B77A5C561934E089",
                "System.Private.CoreLib" => "7CEC85D7BEA7798E",
                "netstandard" => "CC7B13FFCD2DDD51",
                _ => ""
            };
            return expected.Length != 0 && Convert.ToHexString(key) == expected
                && (culture.IsNil || md.GetString(culture).Length == 0)
                && (flags & AssemblyFlags.ContentTypeMask) == 0;
        }
        bool referenceMarker = false;
        string referenceMarkerConstructor = null;
        if (md.IsAssembly)
        {
            var definition = md.GetAssemblyDefinition();
            foreach (var attributeHandle in definition.GetCustomAttributes())
            {
                var attribute = md.GetCustomAttribute(attributeHandle);
                StringHandle name, ns, typeName;
                BlobHandle signature;
                if (attribute.Constructor.Kind == HandleKind.MemberReference)
                {
                    var ctor = md.GetMemberReference((MemberReferenceHandle)attribute.Constructor);
                    if (ctor.Parent.Kind != HandleKind.TypeReference) continue;
                    var type = md.GetTypeReference((TypeReferenceHandle)ctor.Parent);
                    if (type.ResolutionScope.Kind != HandleKind.AssemblyReference) continue;
                    var scope = md.GetAssemblyReference((AssemblyReferenceHandle)type.ResolutionScope);
                    if (!CoreIdentity(scope.Name, scope.PublicKeyOrToken, scope.Culture, scope.Flags)) continue;
                    (name, ns, typeName, signature) = (ctor.Name, type.Namespace, type.Name, ctor.Signature);
                }
                else if (attribute.Constructor.Kind == HandleKind.MethodDefinition)
                {
                    if (!CoreIdentity(definition.Name, definition.PublicKey, definition.Culture, definition.Flags)) continue;
                    var ctor = md.GetMethodDefinition((MethodDefinitionHandle)attribute.Constructor);
                    var type = md.GetTypeDefinition(ctor.GetDeclaringType());
                    if ((ctor.Attributes & MethodAttributes.Static) != 0 || ctor.GetGenericParameters().Count != 0
                        || (ctor.Attributes & (MethodAttributes.SpecialName | MethodAttributes.RTSpecialName))
                            != (MethodAttributes.SpecialName | MethodAttributes.RTSpecialName)
                        || !type.GetDeclaringType().IsNil || type.GetGenericParameters().Count != 0) continue;
                    (name, ns, typeName, signature) = (ctor.Name, type.Namespace, type.Name, ctor.Signature);
                }
                else continue;
                if (md.GetString(name) != ".ctor" || md.GetString(ns) != "System.Runtime.CompilerServices"
                    || md.GetString(typeName) != "ReferenceAssemblyAttribute") continue;
                if (Convert.ToHexString(md.GetBlobBytes(signature)) != "200001"
                    || Convert.ToHexString(md.GetBlobBytes(attribute.Value)) != "01000000")
                    throw new BadImageFormatException("Invalid reference assembly marker evidence.");
                referenceMarker = true;
                referenceMarkerConstructor = Token(attribute.Constructor);
            }
        }
        var errors = new List<object>();
        var methods = new List<object>();
        int count = md.GetTableRowCount(TableIndex.MethodDef);
        for (int rid = 1; rid <= count; rid++)
        {
            var handle = MetadataTokens.MethodDefinitionHandle(rid);
            try
            {
                var method = md.GetMethodDefinition(handle);
                string name = md.GetString(method.Name);
                string owner = TypeName(method.GetDeclaringType());
                int rva = method.RelativeVirtualAddress;
                var code = method.ImplAttributes & MethodImplAttributes.CodeTypeMask;
                string bodyKind;
                if (rva != 0)
                    bodyKind = code == MethodImplAttributes.IL ? "cil" : "native";
                else if ((method.Attributes & (MethodAttributes.Abstract | MethodAttributes.PinvokeImpl)) != 0
                    || code == MethodImplAttributes.Runtime || (method.ImplAttributes & MethodImplAttributes.InternalCall) != 0)
                    bodyKind = "declaration";
                else
                    bodyKind = "missing";
                int? codeSize = null;
                int? exceptionRegions = null;
                string bodyError = null;
                if (bodyKind == "cil")
                {
                    try
                    {
                        var body = pe.GetMethodBody(rva);
                        codeSize = body.GetILContent().Length;
                        exceptionRegions = body.ExceptionRegions.Length;
                    }
                    catch (Exception error)
                    {
                        bodyError = error.GetType().Name + ": " + error.Message;
                    }
                }
                methods.Add(new {
                    token = Token(handle), name, owner, rva, flags = (int)method.Attributes,
                    impl_flags = (int)method.ImplAttributes, body_kind = bodyKind,
                    code_size = codeSize, exception_regions = exceptionRegions, body_error = bodyError
                });
                if (bodyKind == "missing")
                    errors.Add(new { token = Token(handle), error = "Ordinary IL method has no body RVA." });
            }
            catch (Exception error)
            {
                errors.Add(new { token = Token(handle), error = error.GetType().Name + ": " + error.Message });
            }
        }
        var module = md.GetModuleDefinition();
        string assembly = md.IsAssembly ? md.GetString(md.GetAssemblyDefinition().Name) : md.GetString(module.Name);
        return new {
            path, kind = "managed", complete = errors.Count == 0, assembly,
            reader_options = "None", metadata_version = md.MetadataVersion, reference_marker = referenceMarker,
            reference_marker_constructor = referenceMarkerConstructor,
            mvid = md.GetGuid(module.Mvid), method_rows = count, methods, errors
        };
    }

    static void Main(string[] args)
    {
        if (args.Length != 1) throw new ArgumentException("Expected a JSON array of input file paths.");
        var paths = JsonSerializer.Deserialize<string[]>(File.ReadAllText(args[0]));
        foreach (string path in paths)
        {
            object result;
            try
            {
                result = Inspect(path);
            }
            catch (Exception error)
            {
                result = new { path, kind = "unreadable", complete = false, methods = Array.Empty<object>(),
                    errors = new[] { new { error = error.GetType().Name + ": " + error.Message } } };
            }
            Console.WriteLine(JsonSerializer.Serialize(result));
        }
    }
}
