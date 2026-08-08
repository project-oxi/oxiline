/**
 * Swap an item with its neighbor and renormalize the `order` field of every
 * item to be 0..N-1 ascending. No-op at array boundaries.
 */
export function swapOrder<T extends { order: number }>(
  slots: T[],
  i: number,
  dir: -1 | 1,
): T[] {
  const j = i + dir;
  if (j < 0 || j >= slots.length) return slots;
  const next = [...slots];
  [next[i], next[j]] = [next[j], next[i]];
  return next.map((s, k) => ({ ...s, order: k }));
}
