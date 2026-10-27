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

namespace MultidimCases;

public static class Grids
{
    public static int Sum(int[,] grid)
    {
        int sum = 0;
        for (int i = 0; i < grid.GetLength(0); i++)
            for (int j = 0; j < grid.GetLength(1); j++)
                sum += grid[i, j];
        return sum;
    }

    public static int Total(int[,] grid) => grid.Length;

    public static int[,] Build(int rows, int columns) => new int[rows, columns];

    public static int Cell(int[,] grid, int i, int j) => grid[i, j];

    public static void Store(int[,] grid, int i, int j, int value) => grid[i, j] = value;

    public static double SumDouble(double[,] grid) => grid[0, 0] + grid[1, 1];

    public static string Pick(string[,] grid, int i, int j) => grid[i, j];
}
