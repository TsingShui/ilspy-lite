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

using System.Collections.Concurrent;
using System.Reflection.Metadata;
using System.Reflection.PortableExecutable;

using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.Metadata;
using ICSharpCode.Decompiler.TypeSystem;

namespace LiteExperiment;

// A session is single-threaded; the resolver can be called concurrently during type-system loading.
public sealed class ExperimentSession : IDisposable
{
	readonly PEFile module;
	readonly TrackingResolver resolver;
	public CSharpDecompiler Decompiler { get; }
	public string[] MissingReferences => resolver.MissingReferences;
	public KeyValuePair<string, string>[] ResolvedReferences => resolver.ResolvedReferences;

	public ExperimentSession(string assemblyPath, ExperimentProfile profile)
		: this(assemblyPath, Profiles.Settings(profile), d => Profiles.ConfigurePipeline(d, profile))
	{
	}

	public ExperimentSession(string assemblyPath, Cuts cuts)
		: this(assemblyPath, SearchConfiguration.Settings(cuts), d => SearchConfiguration.Configure(d, cuts))
	{
	}

	ExperimentSession(string assemblyPath, ICSharpCode.Decompiler.DecompilerSettings settings,
		Action<CSharpDecompiler> configure)
	{
		module = new PEFile(assemblyPath, PEStreamOptions.PrefetchEntireImage);
		resolver = new TrackingResolver(assemblyPath, module.DetectTargetFrameworkId());
		try
		{
			Decompiler = new CSharpDecompiler(module, resolver, settings);
			configure(Decompiler);
		}
		catch
		{
			Dispose();
			throw;
		}
	}

	public string DecompileType(Type type) =>
		Decompiler.DecompileTypeAsString(new FullTypeName(type.FullName!));

	public string DecompileMethod(Type type, string name)
	{
		var method = type.GetMethod(name) ?? throw new ArgumentException("Method not found: " + name);
		return Decompiler.DecompileAsString(
			System.Reflection.Metadata.Ecma335.MetadataTokens.EntityHandle(method.MetadataToken));
	}

	public string[] Errors => Decompiler.Errors.Select(e => e.ToString()).ToArray();

	public void Dispose()
	{
		resolver.Dispose();
		module.Dispose();
	}

	sealed class TrackingResolver : IAssemblyResolver, IDisposable
	{
		readonly UniversalAssemblyResolver inner;
		readonly ConcurrentBag<MetadataFile> owned = new();
		readonly ConcurrentDictionary<string, string> resolved = new();
		readonly ConcurrentDictionary<string, byte> missing = new();

		public TrackingResolver(string assemblyPath, string targetFramework)
		{
			inner = new UniversalAssemblyResolver(assemblyPath, false, targetFramework,
				streamOptions: PEStreamOptions.PrefetchEntireImage);
			inner.AddSearchDirectory(Path.GetDirectoryName(typeof(object).Assembly.Location)!);
		}

		public string[] MissingReferences => missing.Keys.Order().ToArray();
		public KeyValuePair<string, string>[] ResolvedReferences => resolved.OrderBy(p => p.Key).ToArray();

		MetadataFile? Track(string identity, MetadataFile? file)
		{
			if (file == null)
				missing.TryAdd(identity, 0);
			else
			{
				owned.Add(file);
				resolved[identity] = file.FileName;
			}
			return file;
		}

		public MetadataFile? Resolve(IAssemblyReference reference) =>
			Track(reference.FullName, inner.Resolve(reference));
		public MetadataFile? ResolveModule(MetadataFile mainModule, string moduleName) =>
			Track(mainModule.FileName + "::" + moduleName, inner.ResolveModule(mainModule, moduleName));
		public async Task<MetadataFile?> ResolveAsync(IAssemblyReference reference) =>
			Track(reference.FullName, await inner.ResolveAsync(reference));
		public async Task<MetadataFile?> ResolveModuleAsync(MetadataFile mainModule, string moduleName) =>
			Track(mainModule.FileName + "::" + moduleName, await inner.ResolveModuleAsync(mainModule, moduleName));
		public IDisposable? BeginSnapshot() => inner.BeginSnapshot();

		public void Dispose()
		{
			while (owned.TryTake(out var file))
				file.Dispose();
		}
	}
}
