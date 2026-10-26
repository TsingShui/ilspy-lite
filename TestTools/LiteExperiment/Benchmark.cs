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
public class Benchmark
{
	sealed record Target(string Id, Type Type, string? Method = null);
	sealed record Measurement(double Milliseconds, long AllocatedBytes);
	sealed record Row(string Profile, string Target, string Scope, double MedianMs,
		double MinMs, double MaxMs, long MedianAllocatedBytes, int Characters, int Errors, int Warnings, Measurement[] Samples);

	static readonly Type[] types = {
		typeof(ControlFlowSample), typeof(AsyncSample), typeof(IteratorSample),
		typeof(ClosureSample), typeof(QuerySample), typeof(ExpressionTreeSample), typeof(RecordSample)
	};

	static Target[] Targets() => types.Select(t => new Target(t.Name, t))
		.Concat(new[] {
			new Target("Sum", typeof(ControlFlowSample), nameof(ControlFlowSample.Sum)),
			new Target("Divide", typeof(ControlFlowSample), nameof(ControlFlowSample.Divide)),
			new Target("EvaluationOrder", typeof(ControlFlowSample), nameof(ControlFlowSample.EvaluationOrder)),
			new Target("ReadAsync", typeof(AsyncSample), nameof(AsyncSample.ReadAsync)),
			new Target("Count", typeof(IteratorSample), nameof(IteratorSample.Count)),
			new Target("Positive", typeof(QuerySample), nameof(QuerySample.Positive)),
		}).ToArray();

	static string Decompile(ExperimentSession session, Target target) => target.Method == null
		? session.DecompileType(target.Type) : session.DecompileMethod(target.Type, target.Method);

	static void WriteIL(Target target, string root)
	{
		using var module = new PEFile(ExperimentTests.AssemblyPath);
		using var writer = new StringWriter();
		var disassembler = new ReflectionDisassembler(new PlainTextOutput(writer), CancellationToken.None) {
			DetectControlStructure = false,
			ShowMetadataTokens = true
		};
		if (target.Method == null)
			disassembler.DisassembleType(module, (TypeDefinitionHandle)MetadataTokens.EntityHandle(target.Type.MetadataToken));
		else
			disassembler.DisassembleMethod(module, (MethodDefinitionHandle)MetadataTokens.EntityHandle(
				target.Type.GetMethod(target.Method)!.MetadataToken));
		string directory = Path.Combine(root, "IL");
		Directory.CreateDirectory(directory);
		File.WriteAllText(Path.Combine(directory, target.Id + ".il.txt"), writer.ToString());
	}

	static Measurement Measure(Action action)
	{
		long bytes = GC.GetTotalAllocatedBytes(precise: true);
		long start = Stopwatch.GetTimestamp();
		action();
		return new Measurement(Stopwatch.GetElapsedTime(start).TotalMilliseconds,
			GC.GetTotalAllocatedBytes(precise: true) - bytes);
	}

	static Row Summarize(ExperimentProfile profile, Target target, string scope,
		List<Measurement> measurements, string text, int errors) => new(
		profile.ToString(), target.Id, scope,
		measurements.Select(m => m.Milliseconds).Order().ElementAt(measurements.Count / 2),
		measurements.Min(m => m.Milliseconds), measurements.Max(m => m.Milliseconds),
		measurements.Select(m => m.AllocatedBytes).Order().ElementAt(measurements.Count / 2),
		text.Length, errors, Regex.Matches(text, @"//\s*IL_[0-9a-fA-F]+:").Count, measurements.ToArray());

	[Test, Explicit("Writes a bounded A/B/C timing and output report; not a performance assertion.")]
	public void Run()
	{
		const int rounds = 9;
		var profiles = Enum.GetValues<ExperimentProfile>();
		string root = Path.GetFullPath(Environment.GetEnvironmentVariable("ILSPY_LITE_REPORT") ?? "report/lite");
		Directory.CreateDirectory(root);
		var rows = new List<Row>();
		var configuration = new List<object>();
		var targets = Targets();
		foreach (var target in targets)
		{
			WriteIL(target, root);
			var sessions = new List<ExperimentSession>();
			try
			{
				foreach (var profile in profiles)
					sessions.Add(new ExperimentSession(ExperimentTests.AssemblyPath, profile));
				var warm = profiles.Select(_ => new List<Measurement>()).ToArray();
				var cold = profiles.Select(_ => new List<Measurement>()).ToArray();
				var texts = new string[profiles.Length];
				for (int p = 0; p < profiles.Length; p++)
				{
					for (int i = 0; i < 3; i++)
						texts[p] = Decompile(sessions[p], target);
					Assert.That(sessions[p].MissingReferences, Is.Empty, target.Id);
					Assert.That(sessions[p].Errors, Is.Empty, target.Id);
					string directory = Path.Combine(root, profiles[p].ToString());
					Directory.CreateDirectory(directory);
					string banner = profiles[p] == ExperimentProfile.Full ? "" :
						"// C#-like reverse-engineering output; not guaranteed to compile.\n" +
						"// Compiler-generated names and lowered implementations are intentionally retained.\n";
					File.WriteAllText(Path.Combine(directory, target.Id + ".cs.txt"), banner + texts[p]);
				}
				if (target == targets[0])
				{
					for (int p = 0; p < profiles.Length; p++)
						configuration.Add(new {
							Profile = profiles[p].ToString(),
							Settings = Profiles.Settings(profiles[p]),
							IL = sessions[p].Decompiler.ILTransforms.Select(t => t.GetType().Name),
							AST = sessions[p].Decompiler.AstTransforms.Select(t => t.GetType().Name),
							References = sessions[p].ResolvedReferences
						});
				}
				for (int p = 1; p < profiles.Length; p++)
					Assert.That(sessions[p].ResolvedReferences, Is.EqualTo(sessions[0].ResolvedReferences), target.Id);
				Assert.That(texts[2], Is.EqualTo(texts[1]), target.Id);
				for (int round = 0; round < rounds; round++)
				{
					for (int offset = 0; offset < profiles.Length; offset++)
					{
						int p = (round + offset) % profiles.Length;
						string warmText = "";
						warm[p].Add(Measure(() => warmText = Decompile(sessions[p], target)));
						Assert.That(warmText, Is.EqualTo(texts[p]), target.Id);
						Assert.That(sessions[p].Errors, Is.Empty, target.Id);
						string coldText = "";
						string[] errors = Array.Empty<string>(), missing = Array.Empty<string>();
						cold[p].Add(Measure(() => {
							using var fresh = new ExperimentSession(ExperimentTests.AssemblyPath, profiles[p]);
							coldText = Decompile(fresh, target);
							errors = fresh.Errors;
							missing = fresh.MissingReferences;
						}));
						Assert.That(coldText, Is.EqualTo(texts[p]), target.Id);
						Assert.That(errors, Is.Empty, target.Id);
						Assert.That(missing, Is.Empty, target.Id);
					}
				}
				for (int p = 0; p < profiles.Length; p++)
				{
					rows.Add(Summarize(profiles[p], target, "warm", warm[p], texts[p], sessions[p].Errors.Length));
					rows.Add(Summarize(profiles[p], target, "fresh-session", cold[p], texts[p], sessions[p].Errors.Length));
				}
			}
			finally
			{
				foreach (var session in sessions)
					session.Dispose();
			}
		}
		var options = new JsonSerializerOptions { WriteIndented = true };
		File.WriteAllText(Path.Combine(root, "measurements.json"), JsonSerializer.Serialize(new {
			DateUtc = DateTime.UtcNow, Runtime = System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription,
			OS = System.Runtime.InteropServices.RuntimeInformation.OSDescription,
			AssemblySha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(ExperimentTests.AssemblyPath))),
			EngineSha256 = Convert.ToHexString(SHA256.HashData(File.ReadAllBytes(typeof(ICSharpCode.Decompiler.DecompilerSettings).Assembly.Location))),
			Rounds = rounds, Configuration = configuration, Measurements = rows
		}, options));
		var report = new StringBuilder("# Lite experiment\n\n");
		report.AppendLine("Release .NET 10; 3 warmups, 9 samples, rotating profile order. Times include text output.");
		report.AppendLine("Fresh-session includes PE loading, resolution and type-system construction; OS caches/JIT are warm.");
		report.AppendLine("Allocated bytes are process-wide managed allocations, not retained or peak memory. No speed assertions.");
		report.AppendLine("Type outputs contain different amounts of lowered implementation; do not compare them as equal work.\n");
		report.AppendLine("| Scope | Target | Profile | Median ms | Min-max ms | Allocated KiB | Chars | Errors | IL warnings |");
		report.AppendLine("|---|---|---|---:|---:|---:|---:|---:|---:|");
		foreach (var row in rows)
			report.AppendLine(FormattableString.Invariant($"| {row.Scope} | {row.Target} | {row.Profile} | {row.MedianMs:F3} | {row.MinMs:F3}-{row.MaxMs:F3} | {row.MedianAllocatedBytes / 1024.0:F1} | {row.Characters} | {row.Errors} | {row.Warnings} |"));
		File.WriteAllText(Path.Combine(root, "summary.md"), report.ToString());
		TestContext.Out.WriteLine("Report: " + root);
	}
}
