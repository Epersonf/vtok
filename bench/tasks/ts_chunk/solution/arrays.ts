export function chunk<T>(arr: T[], size: number): T[][] {
  if (!Number.isInteger(size) || size < 1) {
    throw new RangeError('size must be positive');
  }
  const out: T[][] = [];
  for (let i = 0; i < arr.length; i += size) {
    out.push(arr.slice(i, i + size));
  }
  return out;
}

export function unique<T>(arr: T[]): T[] {
  return [...new Set(arr)];
}

export function zip<A, B>(a: A[], b: B[]): [A, B][] {
  const n = Math.min(a.length, b.length);
  const out: [A, B][] = [];
  for (let i = 0; i < n; i++) {
    out.push([a[i], b[i]]);
  }
  return out;
}
