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

using ICSharpCode.Decompiler.IL;
using ICSharpCode.Decompiler.IL.Transforms;

namespace LiteExperiment;

// Flat, deterministic names for lowered methods. Nested functions use the upstream scope logic.
public sealed class SimpleVariableNames : IILTransform
{
	public void Run(ILFunction function, ILTransformContext context)
	{
		if (function.LocalFunctions.Count > 0 || function.Descendants.OfType<ILFunction>().Any(f => f != function))
		{
			new AssignVariableNames().Run(function, context);
			return;
		}
		var reserved = new HashSet<string>(StringComparer.Ordinal);
		foreach (var v in function.Variables)
		{
			if (v.Name != null)
				reserved.Add(v.Name);
		}
		if (function.Method?.DeclaringTypeDefinition is { } type)
		{
			foreach (var member in type.GetMembers())
				reserved.Add(member.Name);
		}
		int index = 0;
		foreach (var v in function.Variables)
		{
			context.CancellationToken.ThrowIfCancellationRequested();
			if (v.Kind is VariableKind.Parameter or VariableKind.InitializerTarget or VariableKind.NamedArgument)
				continue;
			string name;
			do
			{
				name = "__il_v" + index++;
			} while (!reserved.Add(name));
			v.Name = name;
		}
	}
}
