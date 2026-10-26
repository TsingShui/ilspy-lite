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

using System.Linq.Expressions;

namespace LiteSamples;

public static class AsyncSample
{
	public static async Task<int> ReadAsync(int value)
	{
		await Task.Yield();
		return value + 1;
	}
}

public static class IteratorSample
{
	public static IEnumerable<int> Count(int count)
	{
		for (int i = 0; i < count; i++)
			yield return i * 2;
	}
}

public static class ClosureSample
{
	public static Func<int, int> Offset(int offset) => value => value + offset;
}

public static class QuerySample
{
	public static IEnumerable<int> Positive(int[] values) =>
		from value in values where value > 0 select value * 2;
}

public static class ExpressionTreeSample
{
	public static Expression<Func<int, int>> Increment() => value => value + 1;
}

public record RecordSample(int Id, string Name);

public static class ControlFlowSample
{
	static int state;

	public static int Sum(int count)
	{
		int sum = 0;
		for (int i = 0; i < count; i++)
		{
			if ((i & 1) == 0)
				sum += i;
			else
				sum -= i;
		}
		return sum;
	}

	public static int Select(int value)
	{
		switch (value)
		{
			case 0: return 7;
			case 1: return 9;
			case 2: return 13;
			default: return -1;
		}
	}

	public static int Divide(int value)
	{
		try
		{
			return 100 / value;
		}
		catch (DivideByZeroException)
		{
			return -1;
		}
		finally
		{
			state++;
		}
	}

	public static int CheckedAdd(int value) => checked(value + 1);
	public static int Shift(uint value) => (int)(value >> 1);
	public static int Conditional(bool first, int left, int right) => (first ? left : right) + 3;

	static int Next() => ++state;
	static int Pair(int left, int right) => left * 10 + right;

	public static int EvaluationOrder()
	{
		state = 0;
		return Pair(Next(), Next());
	}

	public static int PreserveOldValue()
	{
		int value = 7;
		return Pair(value, value = 1);
	}
}
