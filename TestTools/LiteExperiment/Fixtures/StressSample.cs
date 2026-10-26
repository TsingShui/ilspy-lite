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

namespace LiteSamples;

public static class StressSample
{
	static int __il_v0;

	public static int Run(int __il_v1)
	{
		__il_v0 = 0;
		int sum = 0;
		int[,] cells = { { 1, 2 }, { 3, 4 } };
		foreach (int cell in cells)
			sum += cell;
		for (int i = 0; i < __il_v1; i++)
		{
			if (i % 3 == 0)
				continue;
			if (i > 15)
				break;
			sum += i;
		}
		try
		{
			if (__il_v1 < 0)
				throw new ArgumentException("negative");
			sum += 100 / __il_v1;
		}
		catch (ArgumentException e) when (e.Message == "negative")
		{
			sum -= 7;
		}
		catch (DivideByZeroException)
		{
			sum -= 11;
		}
		finally
		{
			__il_v0++;
		}
		using (var stream = new MemoryStream())
		{
			stream.WriteByte(19);
			sum += (int)stream.Length;
		}
		// This private array deliberately exercises monitor lowering in a decompiler fixture.
#pragma warning disable CA2002
		lock (cells)
		{
			sum += __il_v0;
		}
#pragma warning restore CA2002
		return sum;
	}

	public static int SwitchString(string? text) => text switch {
		"alpha" => 1, "beta" => 2, "gamma" => 3, "delta" => 4,
		"epsilon" => 5, "zeta" => 6, "eta" => 7, "theta" => 8,
		null => -1, _ => 0
	};

	public static int? NullableMath(int? a, int? b) => a + b;

	public static int RefOrder()
	{
		int[] values = { 3, 7 };
		int i = 0;
		values[i++] += values[i++];
		return values[0] * 10 + i;
	}
}

public class PropertySample
{
	public int Value { get; set; }
	public event Action? Changed;
	public void Raise() => Changed?.Invoke();
}
