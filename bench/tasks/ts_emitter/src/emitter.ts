type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of list) {
      fn(...args);
    }
    return true;
  }
}
