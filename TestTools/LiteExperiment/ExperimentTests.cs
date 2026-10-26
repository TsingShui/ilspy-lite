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

using LiteSamples;

using NUnit.Framework;

namespace LiteExperiment;

[TestFixture]
public class ExperimentTests
{
	internal static readonly string AssemblyPath = typeof(AsyncSample).Assembly.Location;

	[TestCase(typeof(IteratorSample), "MoveNext", "yield return")]
	[TestCase(typeof(ClosureSample), "DisplayClass", "=>")]
	[TestCase(typeof(QuerySample), "Enumerable.Where", "select ")]
	[TestCase(typeof(ExpressionTreeSample), "Expression.Lambda", "=>")]
	[TestCase(typeof(RecordSample), "PrintMembers", "record RecordSample")]
	public void LiteTypeRetainsLoweredImplementation(Type type, string required, string forbidden)
	{
		using var session = new ExperimentSession(AssemblyPath, ExperimentProfile.LiteSettings);
		string text = session.DecompileType(type);
		Assert.That(session.Errors, Is.Empty);
		Assert.That(session.MissingReferences, Is.Empty);
		Assert.That(text, Does.Contain(required));
		Assert.That(text, Does.Not.Contain(forbidden));
	}

	[TestCase(typeof(ControlFlowSample))]
	[TestCase(typeof(AsyncSample))]
	[TestCase(typeof(IteratorSample))]
	[TestCase(typeof(ClosureSample))]
	[TestCase(typeof(QuerySample))]
	[TestCase(typeof(ExpressionTreeSample))]
	[TestCase(typeof(RecordSample))]
	public void PrunedTypeMatchesSettingsOnly(Type type)
	{
		using var settings = new ExperimentSession(AssemblyPath, ExperimentProfile.LiteSettings);
		using var pruned = new ExperimentSession(AssemblyPath, ExperimentProfile.LitePruned);
		Assert.That(pruned.DecompileType(type), Is.EqualTo(settings.DecompileType(type)));
		Assert.That(pruned.Errors, Is.Empty);
		Assert.That(settings.Errors, Is.Empty);
		Assert.That(pruned.MissingReferences, Is.Empty);
	}

	[TestCase(nameof(ControlFlowSample.Sum), "for (")]
	[TestCase(nameof(ControlFlowSample.Select), "switch")]
	[TestCase(nameof(ControlFlowSample.Divide), "finally")]
	[TestCase(nameof(ControlFlowSample.CheckedAdd), "checked")]
	[TestCase(nameof(ControlFlowSample.EvaluationOrder), "Pair(Next(), Next())")]
	[TestCase(nameof(ControlFlowSample.PreserveOldValue), "Pair(")]
	public void LiteMethodRetainsBasicOperations(string method, string expected)
	{
		using var full = new ExperimentSession(AssemblyPath, ExperimentProfile.Full);
		using var lite = new ExperimentSession(AssemblyPath, ExperimentProfile.LitePruned);
		string text = lite.DecompileMethod(typeof(ControlFlowSample), method);
		Assert.That(lite.Errors, Is.Empty);
		Assert.That(lite.MissingReferences, Is.Empty);
		Assert.That(text, Does.Contain(expected));
		Assert.That(text, Is.EqualTo(full.DecompileMethod(typeof(ControlFlowSample), method)));
	}

	[Test]
	public void LiteTypeExposesAsyncStateMachine()
	{
		using var session = new ExperimentSession(AssemblyPath, ExperimentProfile.LiteSettings);
		string text = session.DecompileType(typeof(AsyncSample));
		Assert.That(session.Errors, Is.Empty);
		Assert.That(session.MissingReferences, Is.Empty);
		Assert.That(text, Does.Not.Contain("async Task"));
		Assert.That(text, Does.Contain("MoveNext"));
		Assert.That(text, Does.Contain("AwaitUnsafeOnCompleted"));
	}
}
