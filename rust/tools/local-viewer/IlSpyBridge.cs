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
using System.Reflection;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Reflection.PortableExecutable;
using System.Text.Json;
using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.Metadata;

// Developer-only static reader and ILSpy adapter. Never runtime-loads an input assembly.
internal static class IlSpyBridge
{
    static string Version => typeof(CSharpDecompiler).Assembly
        .GetCustomAttribute<AssemblyInformationalVersionAttribute>()?.InformationalVersion ?? "unknown";
    static string Token(EntityHandle handle) => $"0x{MetadataTokens.GetToken(handle):X8}";
    static void Main(string[] args)
    {
        try
        {
            if (args.Length == 1 && args[0] == "version")
            {
                Console.Write(JsonSerializer.Serialize(new { version = Version }));
                return;
            }
            if (args.Length < 2) throw new ArgumentException("Expected catalog or decompile and an assembly path.");
            string path = Path.GetFullPath(args[1]);
            if (args[0] == "catalog")
            {
                using var pe = new PEReader(File.OpenRead(path));
                var md = pe.GetMetadataReader();
                string Name(TypeDefinitionHandle handle)
                {
                    var parts = new List<string>();
                    var seen = new HashSet<TypeDefinitionHandle>();
                    string ns = "";
                    while (!handle.IsNil)
                    {
                        if (parts.Count >= 64 || !seen.Add(handle))
                            throw new BadImageFormatException("Cyclic or excessive nested type path.");
                        var row = md.GetTypeDefinition(handle);
                        parts.Add(md.GetString(row.Name));
                        ns = md.GetString(row.Namespace);
                        handle = row.GetDeclaringType();
                    }
                    parts.Reverse();
                    return (ns.Length == 0 ? "" : ns + ".") + string.Join("/", parts);
                }
                Console.Write(JsonSerializer.Serialize(new {
                    mvid = md.GetGuid(md.GetModuleDefinition().Mvid),
                    types = md.TypeDefinitions.Select(h => {
                        var row = md.GetTypeDefinition(h);
                        return new {
                            token = Token(h), name = Name(h),
                            kind = (row.Attributes & TypeAttributes.Interface) != 0 ? "interface" : "type",
                            parent = row.GetDeclaringType().IsNil ? null : Token(row.GetDeclaringType()),
                            fields = row.GetFields().Count,
                            methods = row.GetMethods().Select(m => new {
                                token = Token(m), name = md.GetString(md.GetMethodDefinition(m).Name)
                            }).ToArray()
                        };
                    }).ToArray()
                }));
                return;
            }
            if (args[0] != "decompile" || args.Length != 4)
                throw new ArgumentException("Expected decompile, assembly, TypeDef token and reference directory.");
            int token = Convert.ToInt32(args[2][2..], 16);
            EntityHandle selected = MetadataTokens.EntityHandle(token);
            if (selected.Kind != HandleKind.TypeDefinition)
                throw new ArgumentException("A TypeDef token is required.");
            using var module = new PEFile(path, PEStreamOptions.PrefetchEntireImage);
            var resolver = new UniversalAssemblyResolver(path, false, module.DetectTargetFrameworkId());
            resolver.AddSearchDirectory(Path.GetDirectoryName(path));
            if (Directory.Exists(args[3])) resolver.AddSearchDirectory(Path.GetFullPath(args[3]));
            resolver.AddSearchDirectory(Path.GetDirectoryName(typeof(object).Assembly.Location));
            var decompiler = new CSharpDecompiler(module, resolver, new DecompilerSettings {
                UseDebugSymbols = false, ShowXmlDocumentation = false
            });
            string text = decompiler.DecompileTypesAsString(new[] { (TypeDefinitionHandle)selected });
            Console.Write(JsonSerializer.Serialize(new {
                version = Version, text, diagnostics = decompiler.Errors.Select(e => e.ToString()).ToArray(),
                errorCount = decompiler.Errors.Count
            }));
        }
        catch (Exception error)
        {
            Console.Error.WriteLine(error.GetType().Name + ": " + error.Message);
            Environment.ExitCode = 1;
        }
    }
}
