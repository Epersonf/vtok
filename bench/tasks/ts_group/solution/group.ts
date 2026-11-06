export function sum(xs: number[]): number {
  let total = 0;
  for (const x of xs) {
    total += x;
  }
  return total;
}

export function groupBy<T>(arr: T[], key: (x: T) => string): Record<string, T[]> {
  const out: Record<string, T[]> = {};
  for (const x of arr) {
    const k = key(x);
    (out[k] ??= []).push(x);
  }
  return out;
}

export function countBy<T>(arr: T[], key: (x: T) => string): Record<string, number> {
  const out: Record<string, number> = {};
  for (const x of arr) {
    const k = key(x);
    out[k] = (out[k] ?? 0) + 1;
  }
  return out;
}

export function partition<T>(arr: T[], pred: (x: T) => boolean): [T[], T[]] {
  const yes: T[] = [];
  const no: T[] = [];
  for (const x of arr) {
    (pred(x) ? yes : no).push(x);
  }
  return [yes, no];
}
