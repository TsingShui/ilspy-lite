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
using ICSharpCode.Decompiler.CSharp.Transforms;
using ICSharpCode.Decompiler.IL.ControlFlow;
using ICSharpCode.Decompiler.IL.Transforms;

namespace LiteExperiment;

public enum ExperimentProfile
{
	Full,
	LiteSettings,
	LitePruned
}

public static class Profiles
{
	public static DecompilerSettings Settings(ExperimentProfile profile)
	{
		bool sugar = profile == ExperimentProfile.Full;
		return new DecompilerSettings {
			AsyncAwait = sugar,
			AwaitInCatchFinally = sugar,
			AsyncEnumerator = sugar,
			YieldReturn = sugar,
			AnonymousMethods = sugar,
			AnonymousTypes = sugar,
			UseLambdaSyntax = sugar,
			LocalFunctions = sugar,
			ExpressionTrees = sugar,
			QueryExpressions = sugar,
			ExtensionMethods = sugar,
			RecordClasses = sugar,
			RecordStructs = sugar,
			UsePrimaryConstructorSyntax = sugar,
			UsePrimaryConstructorSyntaxForNonRecordTypes = sugar,
			UseDebugSymbols = false,
			ShowXmlDocumentation = false,
			UseRefLocalsForAccurateOrderOfEvaluation = true
		};
	}

	public static void ConfigurePipeline(CSharpDecompiler decompiler, ExperimentProfile profile)
	{
		if (profile != ExperimentProfile.LitePruned)
			return;
		// These passes are disabled by the same profile before type-system construction.
		// Preserve the order and nested block/statement pipelines of every remaining pass.
		foreach (var transform in decompiler.ILTransforms.ToArray())
		{
			if (transform is YieldReturnDecompiler or AsyncAwaitDecompiler
				or DelegateConstruction or LocalFunctionDecompiler)
				decompiler.ILTransforms.Remove(transform);
		}
		foreach (var transform in decompiler.AstTransforms.ToArray())
		{
			if (transform is IntroduceQueryExpressions or CombineQueryExpressions or AddXmlDocumentationTransform)
				decompiler.AstTransforms.Remove(transform);
		}
	}
}
