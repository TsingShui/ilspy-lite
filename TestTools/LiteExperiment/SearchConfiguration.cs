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

using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.IL.Transforms;

namespace LiteExperiment;

[Flags]
public enum Cuts
{
	None = 0,
	SimpleNames = 1,
	NoAstPatterns = 2,
	NoAssignmentSugar = 4,
	NoLateCleanup = 8,
	NoSroa = 16,
	QualifiedNames = 32,
	All = 63
}

public static class SearchConfiguration
{
	// Bounded corpus recommendations, not universal optima.
	public const Cuts RecommendedClass = Cuts.SimpleNames | Cuts.NoAstPatterns | Cuts.NoLateCleanup | Cuts.NoSroa;
	public const Cuts RecommendedMethod = RecommendedClass | Cuts.QualifiedNames;

	public static DecompilerSettings Settings(Cuts cuts)
	{
		if ((cuts & ~Cuts.All) != 0)
			throw new ArgumentOutOfRangeException(nameof(cuts));
		var settings = Profiles.Settings(ExperimentProfile.LitePruned);
		// All candidates expose the same fields/accessors, including without AST patterns.
		settings.AutomaticProperties = false;
		settings.GetterOnlyAutomaticProperties = false;
		settings.AutomaticEvents = false;
		settings.FieldKeyword = false;
		if (cuts.HasFlag(Cuts.QualifiedNames))
			settings.UsingDeclarations = false;
		if (cuts.HasFlag(Cuts.NoAssignmentSugar))
			settings.IntroduceIncrementAndDecrement = false;
		return settings;
	}

	public static void Configure(CSharpDecompiler decompiler, Cuts cuts)
	{
		Profiles.ConfigurePipeline(decompiler, ExperimentProfile.LitePruned);
		if (cuts.HasFlag(Cuts.SimpleNames))
		{
			int index = decompiler.ILTransforms.ToList().FindIndex(t => t is AssignVariableNames);
			if (index < 0)
				throw new InvalidOperationException("Variable naming pass not found.");
			decompiler.ILTransforms[index] = new SimpleVariableNames();
		}
		if (cuts.HasFlag(Cuts.NoAstPatterns))
			RemoveAst(decompiler, "PatternStatementTransform");
		if (cuts.HasFlag(Cuts.NoAssignmentSugar))
			RemoveAst(decompiler, "PrettifyAssignments");
		if (cuts.HasFlag(Cuts.NoLateCleanup))
		{
			RemoveIL(decompiler, "ReduceNestingTransform");
			RemoveIL(decompiler, "RemoveRedundantReturn");
		}
		if (cuts.HasFlag(Cuts.NoSroa))
			RemoveIL(decompiler, "TransformDisplayClassUsage");
	}

	// Some upstream transforms are internal. Fail on missing or ambiguous pass names.
	static void RemoveIL(CSharpDecompiler decompiler, string name) =>
		decompiler.ILTransforms.Remove(decompiler.ILTransforms.Single(t => t.GetType().Name == name));

	static void RemoveAst(CSharpDecompiler decompiler, string name) =>
		decompiler.AstTransforms.Remove(decompiler.AstTransforms.Single(t => t.GetType().Name == name));
}
