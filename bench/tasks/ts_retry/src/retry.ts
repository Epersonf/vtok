// Retry helper

export async function retry<T>(fn: () => Promise<T>, attempts: number): Promise<T> {
  return fn();
}
