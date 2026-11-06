// Array helpers

export function chunk<T>(arr: T[], size: number): T[][] {
  const out: T[][] = [];
  let i = 0;
  while (i < arr.length) {
    out.push(arr.slice(i, i + size));
  }
  return out;
}
