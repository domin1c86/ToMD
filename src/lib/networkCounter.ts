/**
 * Session-scoped count of provider network requests, shown in the sidebar
 * privacy block. Incremented only by the desktop wrappers that cause
 * network traffic (connection tests, analysis runs, rule refinement).
 */
let count = 0;
const listeners = new Set<() => void>();

export function recordNetworkRequest(): void {
  count += 1;
  listeners.forEach((listener) => listener());
}

export function getNetworkRequestCount(): number {
  return count;
}

export function subscribeNetworkRequests(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
