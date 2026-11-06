export function identity(n: number): number[][] {
  if (n < 1) {
    throw new RangeError('n must be positive');
  }
  const out: number[][] = [];
  for (let i = 0; i < n; i++) {
    const row: number[] = [];
    for (let j = 0; j < n; j++) {
      row.push(i === j ? 1 : 0);
    }
    out.push(row);
  }
  return out;
}

export function transpose(m: number[][]): number[][] {
  if (m.length === 0) {
    return [];
  }
  return m[0].map((_, j) => m.map((row) => row[j]));
}

export function multiply(a: number[][], b: number[][]): number[][] {
  const cols = a.length ? a[0].length : 0;
  if (cols !== b.length) {
    throw new Error('dimension mismatch');
  }
  const width = b.length ? b[0].length : 0;
  const out: number[][] = [];
  for (let i = 0; i < a.length; i++) {
    const row: number[] = [];
    for (let j = 0; j < width; j++) {
      let sum = 0;
      for (let k = 0; k < cols; k++) {
        sum += a[i][k] * b[k][j];
      }
      row.push(sum);
    }
    out.push(row);
  }
  return out;
}
