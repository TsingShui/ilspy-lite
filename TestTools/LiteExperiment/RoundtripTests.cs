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
using System.Runtime.Loader;

using LiteSamples;

using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;

using NUnit.Framework;

namespace LiteExperiment;

[TestFixture]
public class RoundtripTests
{
	[TestCase(ExperimentProfile.Full)]
	[TestCase(ExperimentProfile.LiteSettings)]
	[TestCase(ExperimentProfile.LitePruned)]
	public void BasicControlFlowRecompilesAndPreservesResults(ExperimentProfile profile)
	{
		using var session = new ExperimentSession(ExperimentTests.AssemblyPath, profile);
		string source = session.DecompileType(typeof(ControlFlowSample));
		Assert.That(session.Errors, Is.Empty);
		VerifyControlFlow(source);
	}

	internal static void VerifyControlFlow(string source)
	{
		var references = ((string?)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES"))!
			.Split(Path.PathSeparator).Select(p => MetadataReference.CreateFromFile(p));
		var compilation = CSharpCompilation.Create("Roundtrip_ControlFlow",
			new[] { CSharpSyntaxTree.ParseText(source) }, references,
			new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));
		using var stream = new MemoryStream();
		var result = compilation.Emit(stream);
		Assert.That(result.Success, Is.True, string.Join(Environment.NewLine, result.Diagnostics));
		stream.Position = 0;
		var context = new AssemblyLoadContext("roundtrip", isCollectible: true);
		try
		{
			var type = context.LoadFromStream(stream).GetType(typeof(ControlFlowSample).FullName!)!;
			object? Call(string name, params object[] arguments) => type.GetMethod(name)!.Invoke(null, arguments);
			Assert.That(Call(nameof(ControlFlowSample.Sum), 6), Is.EqualTo(-3));
			Assert.That(Call(nameof(ControlFlowSample.Select), 2), Is.EqualTo(13));
			Assert.That(Call(nameof(ControlFlowSample.Select), 9), Is.EqualTo(-1));
			Assert.That(Call(nameof(ControlFlowSample.Divide), 0), Is.EqualTo(-1));
			Assert.That(Call(nameof(ControlFlowSample.Divide), 4), Is.EqualTo(25));
			Assert.That(type.GetField("state", BindingFlags.Static | BindingFlags.NonPublic)!.GetValue(null), Is.EqualTo(2));
			Assert.That(Call(nameof(ControlFlowSample.Shift), uint.MaxValue), Is.EqualTo(int.MaxValue));
			Assert.That(Call(nameof(ControlFlowSample.Conditional), true, 7, 9), Is.EqualTo(10));
			Assert.That(Call(nameof(ControlFlowSample.Conditional), false, 7, 9), Is.EqualTo(12));
			Assert.That(Call(nameof(ControlFlowSample.EvaluationOrder)), Is.EqualTo(12));
			Assert.That(Call(nameof(ControlFlowSample.PreserveOldValue)), Is.EqualTo(71));
			var error = Assert.Throws<TargetInvocationException>(() => Call(nameof(ControlFlowSample.CheckedAdd), int.MaxValue));
			Assert.That(error!.InnerException, Is.TypeOf<OverflowException>());
		}
		finally
		{
			context.Unload();
		}
	}
}
