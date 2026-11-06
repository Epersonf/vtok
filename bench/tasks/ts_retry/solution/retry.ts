export async function retry<T>(
  fn: () => Promise<T>,
  attempts: number,
  onRetry?: (error: unknown, attempt: number) => void,
): Promise<T> {
  if (attempts < 1) {
    throw new RangeError('attempts must be >= 1');
  }
  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await fn();
    } catch (e) {
      lastError = e;
      if (attempt < attempts && onRetry) {
        onRetry(e, attempt);
      }
    }
  }
  throw lastError;
}
