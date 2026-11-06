// Small matrix helpers

export function identity(n: number): number[][] {
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
