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

using System.Diagnostics;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.Disassembler;
using ICSharpCode.Decompiler.Metadata;

using LiteSamples;

using NUnit.Framework;

namespace LiteExperiment;

[TestFixture]
public class CombinationSearch
{
	sealed record Target(string Id, Type Type, string? Method = null)
	{
		public string Path => Type.Assembly.Location;
		public string Decompile(ExperimentSession session) => Method == null
			? session.DecompileType(Type) : session.DecompileMethod(Type, Method);
	}

	sealed record Sample(int Round, double Ms, long Bytes);
	sealed record Row(int Mask, string Cuts, string Target, string Scope, double MedianMs,
		long MedianBytes, int Characters, Sample[] Samples);

	static readonly Target[] classes = SearchTests.FixtureTypes.Select(t => new Target(t.Name, t)).ToArray();
	static readonly Target[] methods = {
		new("Sum", typeof(ControlFlowSample), nameof(ControlFlowSample.Sum)),
		new("Divide", typeof(ControlFlowSample), nameof(ControlFlowSample.Divide)),
		new("EvaluationOrder", typeof(ControlFlowSample), nameof(ControlFlowSample.EvaluationOrder)),
		new("ReadAsync", typeof(AsyncSample), nameof(AsyncSample.ReadAsync)),
		new("Count", typeof(IteratorSample), nameof(IteratorSample.Count)),
		new("Positive", typeof(QuerySample), nameof(QuerySample.Positive)),
		new("StressRun", typeof(StressSample), nameof(StressSample.Run)),
		new("RefOrder", typeof(StressSample), nameof(StressSample.RefOrder))
	};
	static readonly Target[] holdouts = {
		new("MemoryStream", typeof(MemoryStream)), new("StringBuilder", typeof(StringBuilder)),
		new("List", typeof(List<>)), new("Dictionary", typeof(Dictionary<,>))
	};

	static string Root(string stage)
	{
		string path = System.IO.Path.GetFullPath(Environment.GetEnvironmentVariable("ILSPY_SEARCH_REPORT") ?? "report/search-" + stage);
		Directory.CreateDirectory(path);
		return path;
	}

	static Sample Measure(int round, Action action)
	{
		long bytes = GC.GetTotalAllocatedBytes(true);
		long start = Stopwatch.GetTimestamp();
		action();
		double ms = Stopwatch.GetElapsedTime(start).TotalMilliseconds;
		return new(round, ms, GC.GetTotalAllocatedBytes(true) - bytes);
	}

	static Row Summarize(Cuts cuts, string target, string scope, List<Sample> samples, int characters) =>
		new((int)cuts, cuts.ToString(), target, scope, samples.Select(s => s.Ms).Order().ElementAt(samples.Count / 2),
			samples.Select(s => s.Bytes).Order().ElementAt(samples.Count / 2), characters, samples.ToArray());

	static void Check(ExperimentSession session, string id)
	{
		Assert.That(session.Errors, Is.Empty, id);
		Assert.That(session.MissingReferences, Is.Empty, id);
	}

	static string[] Batch(ExperimentSession session, Target[] targets) => targets.Select(t => t.Decompile(session)).ToArray();

	static void SaveOutput(string root, Cuts cuts, Target target, string source)
	{
		string directory = System.IO.Path.Combine(root, "outputs", ((int)cuts).ToString("D2"));
		Directory.CreateDirectory(directory);
		File.WriteAllText(System.IO.Path.Combine(directory, target.Id + ".cs.txt"),
			"// C#-like reverse-engineering output; not guaranteed to compile.\n" + source);
	}

	static void SaveIL(string root, Target target)
	{
		string directory = System.IO.Path.Combine(root, "IL");
		Directory.CreateDirectory(directory);
		using var module = new PEFile(target.Path);
		using var writer = new StringWriter();
		var disassembler = new ReflectionDisassembler(new PlainTextOutput(writer), CancellationToken.None) {
			DetectControlStructure = false, ShowMetadataTokens = true
		};
		if (target.Method == null)
			disassembler.DisassembleType(module, (TypeDefinitionHandle)MetadataTokens.EntityHandle(target.Type.MetadataToken));
		else
			disassembler.DisassembleMethod(module, (MethodDefinitionHandle)MetadataTokens.EntityHandle(target.Type.GetMethod(target.Method)!.MetadataToken));
		File.WriteAllText(System.IO.Path.Combine(directory, target.Id + ".il.txt"), writer.ToString());
	}

	static void Report(string root, string stage, IEnumerable<Cuts> cuts, IEnumerable<Target> targets,
		List<Row> rows, List<object> evidence)
	{
		var config = cuts.Select(c => {
			using var session = new ExperimentSession(ExperimentTests.AssemblyPath, c);
			return new {
				Mask = (int)c, Cuts = c.ToString(), Settings = SearchConfiguration.Settings(c),
				IL = session.Decompiler.ILTransforms.Select(t => t.GetType().Name).ToArray(),
				AST = session.Decompiler.AstTransforms.Select(t => t.GetType().Name).ToArray()
			};
		}).ToArray();
		string Hash(string path) => Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(path)));
		File.WriteAllText(System.IO.Path.Combine(root, "measurements.json"), JsonSerializer.Serialize(new {
			Stage = stage, DateUtc = DateTime.UtcNow,
			Runtime = System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription,
			OS = System.Runtime.InteropServices.RuntimeInformation.OSDescription,
			TieredCompilation = Environment.GetEnvironmentVariable("DOTNET_TieredCompilation"),
			EngineSha256 = Hash(typeof(DecompilerSettings).Assembly.Location),
			HarnessSha256 = Hash(typeof(CombinationSearch).Assembly.Location),
			Inputs = targets.Select(t => t.Path).Distinct().Select(p => new { Path = p, Sha256 = Hash(p) }).ToArray(),
			Configuration = config, Evidence = evidence, Measurements = rows
		}, new JsonSerializerOptions { WriteIndented = true }));
		var text = new StringBuilder("# " + stage + "\n\n");
		text.AppendLine("Times include text output; bytes are process-wide allocation, not retained or peak memory.");
		text.AppendLine("Fresh sessions have warm OS caches/JIT. All masks share member-visibility settings.\n");
		text.AppendLine("| Mask | Target | Scope | Median ms (dimensionless for ratio rows) | Allocated KiB | Chars |");
		text.AppendLine("|---:|---|---|---:|---:|---:|");
		foreach (var row in rows.OrderBy(r => r.Scope).ThenBy(r => r.Target).ThenBy(r => r.MedianMs))
			text.AppendLine(FormattableString.Invariant($"| {row.Mask} | {row.Target} | {row.Scope} | {row.MedianMs:F3} | {row.MedianBytes / 1024.0:F1} | {row.Characters} |"));
		File.WriteAllText(System.IO.Path.Combine(root, "summary.md"), text.ToString());
		TestContext.Out.WriteLine("Report: " + root);
	}

	[Test, Explicit("Screens all 64 masks with paired baseline measurements.")]
	public void Screen()
	{
		const int rounds = 7;
		string root = Root("screen");
		var masks = SearchTests.Candidates.ToArray();
		var rows = new List<Row>();
		var evidence = new List<object>();
		var data = new Dictionary<(Cuts, string), List<Sample>>();
		var sizes = new Dictionary<(Cuts, string), int>();
		var random = new Random(1729);
		foreach (var c in masks)
			new SearchTests().CandidatePreservesMembersAndExecutes(c);
		foreach (var t in classes.Concat(methods))
			SaveIL(root, t);
		for (int round = 0; round < rounds; round++)
		{
			foreach (var mask in masks.OrderBy(_ => random.Next()))
			{
				using var baseline = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.None);
				using var candidate = new ExperimentSession(ExperimentTests.AssemblyPath, mask);
				foreach (var (targets, name, repetitions) in new[] { (classes, "classes", 2), (methods, "methods", 5) })
				{
					string[] expected = Batch(candidate, targets), expectedBase = Batch(baseline, targets);
					Batch(candidate, targets);
					Batch(baseline, targets);
					Sample MeasureBatch(ExperimentSession session, string[] expectedOutput)
					{
						string[][] output = new string[repetitions][];
						var sample = Measure(round, () => {
							for (int n = 0; n < repetitions; n++)
								output[n] = Batch(session, targets);
						});
						foreach (var text in output)
							Assert.That(text, Is.EqualTo(expectedOutput));
						Check(session, mask.ToString());
						return sample with { Ms = sample.Ms / repetitions, Bytes = sample.Bytes / repetitions };
					}
					Sample a, b;
					if ((round + (int)mask) % 2 == 0)
					{
						a = MeasureBatch(candidate, expected);
						b = MeasureBatch(baseline, expectedBase);
					}
					else
					{
						b = MeasureBatch(baseline, expectedBase);
						a = MeasureBatch(candidate, expected);
					}
					void Add(string scope, Sample sample, int characters)
					{
						if (!data.TryGetValue((mask, scope), out var samples))
							data[(mask, scope)] = samples = new List<Sample>();
						samples.Add(sample);
						sizes[(mask, scope)] = characters;
					}
					Add(name, a, expected.Sum(s => s.Length));
					Add(name + "-paired-baseline", b, expectedBase.Sum(s => s.Length));
					Add(name + "-ratio", new(round, a.Ms / b.Ms, 0), 0);
					Assert.That(candidate.ResolvedReferences, Is.EqualTo(baseline.ResolvedReferences));
					if (round == 0)
					{
						for (int t = 0; t < targets.Length; t++)
						{
							Assert.That(Regex.Matches(expected[t], @"//\s*IL_[0-9a-fA-F]+:").Count,
								Is.EqualTo(Regex.Matches(expectedBase[t], @"//\s*IL_[0-9a-fA-F]+:").Count));
							SaveOutput(root, mask, targets[t], expected[t]);
						}
						evidence.Add(new { Mask = (int)mask, Workload = name, References = candidate.ResolvedReferences });
					}
				}
			}
		}
		foreach (var ((mask, scope), samples) in data)
		{
			string workload = scope.StartsWith("classes", StringComparison.Ordinal) ? "classes" : "methods";
			rows.Add(Summarize(mask, workload, scope, samples, sizes[(mask, scope)]));
		}
		Report(root, "screen", masks, classes.Concat(methods), rows, evidence);
	}

	[Test, Explicit("Remeasures selected masks per target, including held-out framework classes.")]
	public void Validate()
	{
		const int rounds = 11;
		string root = Root("validate");
		string list = Environment.GetEnvironmentVariable("ILSPY_SEARCH_MASKS") ?? "0,1,3,7,15,31,63";
		var masks = list.Split(',').Select(s => (Cuts)int.Parse(s)).Prepend(Cuts.None).Distinct().ToArray();
		var targets = classes.Concat(methods).Concat(holdouts).ToArray();
		var rows = new List<Row>();
		var evidence = new List<object>();
		foreach (var mask in masks)
			new SearchTests().CandidatePreservesMembersAndExecutes(mask);
		foreach (var target in targets)
		{
			SaveIL(root, target);
			var sessions = new List<ExperimentSession>();
			try
			{
				foreach (var mask in masks)
					sessions.Add(new ExperimentSession(target.Path, mask));
				var texts = new string[masks.Length];
				var warm = masks.Select(_ => new List<Sample>()).ToArray();
				var fresh = masks.Select(_ => new List<Sample>()).ToArray();
				string[]? members = null;
				for (int p = 0; p < masks.Length; p++)
				{
					if (target.Method == null)
					{
						var current = SearchTests.Members(sessions[p], target.Type);
						members ??= current;
						Assert.That(current, Is.EqualTo(members), target.Id);
					}
					for (int w = 0; w < 4; w++)
						texts[p] = target.Decompile(sessions[p]);
					Check(sessions[p], target.Id);
					Assert.That(sessions[p].ResolvedReferences, Is.EqualTo(sessions[0].ResolvedReferences), target.Id);
					int warnings = Regex.Matches(texts[p], @"//\s*IL_[0-9a-fA-F]+:").Count;
					Assert.That(warnings, Is.EqualTo(Regex.Matches(texts[0], @"//\s*IL_[0-9a-fA-F]+:").Count), target.Id);
					SaveOutput(root, masks[p], target, texts[p]);
					evidence.Add(new { Mask = (int)masks[p], target.Id, Warnings = warnings, Errors = sessions[p].Errors,
						Missing = sessions[p].MissingReferences, References = sessions[p].ResolvedReferences, Members = members });
				}
				for (int round = 0; round < rounds; round++)
				{
					for (int offset = 0; offset < masks.Length; offset++)
					{
						int p = (round + offset) % masks.Length;
						string output = "";
						warm[p].Add(Measure(round, () => output = target.Decompile(sessions[p])));
						Assert.That(output, Is.EqualTo(texts[p]));
						Check(sessions[p], target.Id);
						string[] errors = Array.Empty<string>(), missing = Array.Empty<string>();
						KeyValuePair<string, string>[] references = Array.Empty<KeyValuePair<string, string>>();
						fresh[p].Add(Measure(round, () => {
							using var session = new ExperimentSession(target.Path, masks[p]);
							output = target.Decompile(session);
							errors = session.Errors;
							missing = session.MissingReferences;
							references = session.ResolvedReferences;
						}));
						Assert.That(output, Is.EqualTo(texts[p]));
						Assert.That(errors, Is.Empty);
						Assert.That(missing, Is.Empty);
						Assert.That(references, Is.EqualTo(sessions[p].ResolvedReferences));
					}
				}
				for (int p = 0; p < masks.Length; p++)
				{
					rows.Add(Summarize(masks[p], target.Id, "warm", warm[p], texts[p].Length));
					rows.Add(Summarize(masks[p], target.Id, "fresh-session", fresh[p], texts[p].Length));
				}
			}
			finally
			{
				foreach (var session in sessions)
					session.Dispose();
			}
		}
		Report(root, "validate", masks, targets, rows, evidence);
	}
}
