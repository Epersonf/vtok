type Listener = (...args: any[]) => void;

export class Emitter {
  private handlers: Map<string, Listener[]> = new Map();

  on(event: string, fn: Listener): void {
    const list = this.handlers.get(event) ?? [];
    list.push(fn);
    this.handlers.set(event, list);
  }

  once(event: string, fn: Listener): void {
    const wrapper: Listener = (...args) => {
      this.off(event, wrapper);
      fn(...args);
    };
    this.on(event, wrapper);
  }

  off(event: string, fn: Listener): boolean {
    const list = this.handlers.get(event);
    if (!list) {
      return false;
    }
    const i = list.indexOf(fn);
    if (i < 0) {
      return false;
    }
    list.splice(i, 1);
    return true;
  }

  listenerCount(event: string): number {
    return (this.handlers.get(event) ?? []).length;
  }

  emit(event: string, ...args: any[]): boolean {
    const list = this.handlers.get(event);
    if (!list || list.length === 0) {
      return false;
    }
    for (const fn of [...list]) {
      fn(...args);
    }
    return true;
  }
}
