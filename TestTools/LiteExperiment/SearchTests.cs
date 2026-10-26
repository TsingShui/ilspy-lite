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

using System.Reflection;
using System.Reflection.Metadata.Ecma335;
using System.Runtime.Loader;

using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.TypeSystem;

using LiteSamples;

using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;

using NUnit.Framework;

namespace LiteExperiment;

[TestFixture]
public class SearchTests
{
	public static IEnumerable<Cuts> Candidates => Enumerable.Range(0, 64).Select(i => (Cuts)i);

	internal static readonly Type[] FixtureTypes = {
		typeof(ControlFlowSample), typeof(StressSample), typeof(AsyncSample), typeof(IteratorSample),
		typeof(ClosureSample), typeof(QuerySample), typeof(ExpressionTreeSample), typeof(RecordSample), typeof(PropertySample)
	};

	internal static string[] Members(ExperimentSession session, Type type)
	{
		var tree = session.Decompiler.DecompileType(new FullTypeName(type.FullName!));
		return tree.Descendants.Where(n => n is EntityDeclaration or VariableInitializer)
			.Select(n => n.GetSymbol()).OfType<IEntity>().Where(e => !e.MetadataToken.IsNil)
			.Select(e => MetadataTokens.GetToken(e.MetadataToken).ToString("X8"))
			.Distinct().Order().ToArray();
	}

	[TestCaseSource(nameof(Candidates))]
	public void CandidatePreservesMembersAndExecutes(Cuts cuts)
	{
		using var baseline = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.None);
		using var candidate = new ExperimentSession(ExperimentTests.AssemblyPath, cuts);
		foreach (var type in FixtureTypes)
		{
			var expected = Members(baseline, type);
			Assert.That(expected.Length, Is.GreaterThan(1), type.Name);
			Assert.That(Members(candidate, type), Is.EqualTo(expected), type.Name);
			Assert.That(candidate.Errors, Is.Empty, type.Name);
		}
		Assert.That(candidate.MissingReferences, Is.Empty);
		Assert.That(candidate.ResolvedReferences, Is.EqualTo(baseline.ResolvedReferences));
		RoundtripTests.VerifyControlFlow(candidate.DecompileType(typeof(ControlFlowSample)));
		VerifyStress(candidate.DecompileType(typeof(StressSample)));
		string asyncText = candidate.DecompileType(typeof(AsyncSample));
		Assert.That(asyncText, Does.Contain("MoveNext").And.Contain("AwaitUnsafeOnCompleted"));
		Assert.That(candidate.DecompileType(typeof(IteratorSample)), Does.Contain("MoveNext"));
	}

	internal static void VerifyStress(string source)
	{
		var references = ((string)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES")!)
			.Split(Path.PathSeparator).Select(p => MetadataReference.CreateFromFile(p));
		var compilation = CSharpCompilation.Create("Roundtrip_Stress",
			new[] { CSharpSyntaxTree.ParseText(source) }, references,
			new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));
		using var stream = new MemoryStream();
		var result = compilation.Emit(stream);
		Assert.That(result.Success, Is.True, string.Join(Environment.NewLine, result.Diagnostics));
		stream.Position = 0;
		var context = new AssemblyLoadContext("stress", isCollectible: true);
		try
		{
			var type = context.LoadFromStream(stream).GetType(typeof(StressSample).FullName!)!;
			object? Call(string name, params object?[] args) => type.GetMethod(name)!.Invoke(null, args);
			foreach (int n in new[] { -3, 0, 1, 5, 12, 24 })
				Assert.That(Call(nameof(StressSample.Run), n), Is.EqualTo(StressSample.Run(n)));
			foreach (string? s in new[] { null, "alpha", "beta", "theta", "other" })
				Assert.That(Call(nameof(StressSample.SwitchString), s), Is.EqualTo(StressSample.SwitchString(s)));
			Assert.That(Call(nameof(StressSample.NullableMath), 2, 3), Is.EqualTo(5));
			Assert.That(Call(nameof(StressSample.NullableMath), null, 3), Is.Null);
			Assert.That(Call(nameof(StressSample.RefOrder)), Is.EqualTo(102));
		}
		finally
		{
			context.Unload();
		}
	}

	[Test]
	public void SearchBaselineKeepsBackingFieldsVisible()
	{
		using var session = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.None);
		Assert.That(session.DecompileType(typeof(PropertySample)), Does.Contain("<Value>k__BackingField"));
	}

	[Test]
	public void QualifiedNamesAreExplicit()
	{
		using var session = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.QualifiedNames);
		Assert.That(session.DecompileType(typeof(ExpressionTreeSample)), Does.Contain("System.Linq.Expressions.Expression"));
	}

	[Test]
	public void NoAssignmentSugarKeepsAdditionExplicit()
	{
		using var session = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.NoAssignmentSugar);
		Assert.That(session.DecompileMethod(typeof(ControlFlowSample), nameof(ControlFlowSample.Sum)), Does.Not.Contain("i++"));
	}

	[Test]
	public void SimpleNamesRetainsOperationsWithUniqueLocalNames()
	{
		using var session = new ExperimentSession(ExperimentTests.AssemblyPath, Cuts.SimpleNames);
		string text = session.DecompileMethod(typeof(ControlFlowSample), nameof(ControlFlowSample.Sum));
		Assert.That(session.Errors, Is.Empty);
		Assert.That(text, Does.Contain("__il_v"));
		Assert.That(text, Does.Contain("return"));
	}
}
