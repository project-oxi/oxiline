/**
 * Column-packing layout for overlapping time-blocks (Google-Calendar style).
 *
 * The timeline positions each block by its start/end minute on the Y axis.
 * When two blocks share a time span they would otherwise paint on top of each
 * other. `packColumns` resolves that by grouping overlapping blocks into a
 * **cluster** (a connected component under interval-overlap) and assigning each
 * block a **column** within that cluster via greedy graph coloring — every pair
 * of blocks that still overlap get different columns. The block then renders
 * as `left: (col/cols)*100%`, `width: (1/cols)*100%`, so a cluster of N
 * mutually-overlapping blocks fans out into N side-by-side slivers.
 *
 * Y position (time) is never distorted — only the horizontal share changes.
 * Non-overlapping blocks keep `cols: 1` (full width), identical to today.
 */

export interface Rect {
  start: number;
  end: number;
}

export interface PackedItem {
  /** 0-based column index within the block's cluster. */
  col: number;
  /** Total columns in the block's cluster (width divisor). */
  cols: number;
}

/**
 * Assign each item a `{ col, cols }`. Output is aligned 1:1 with the input
 * array (input order preserved); internal sort is by `start` then input index.
 *
 * Two intervals overlap when `a.start < b.end && b.start < a.end` (half-open),
 * so blocks that merely touch end-to-end do NOT consume an extra column.
 */
export function packColumns<T extends Rect>(items: T[]): PackedItem[] {
  const n = items.length;
  if (n === 0) return [];
  const out: PackedItem[] = new Array(n);

  // Sort indices by start; ties broken by original index for determinism.
  const order = items
    .map((it, idx) => ({ idx, start: it.start, end: it.end }))
    .sort((a, b) => a.start - b.start || a.idx - b.idx);

  let i = 0;
  while (i < n) {
    // Grow a cluster: keep absorbing while the next block starts before the
    // running max end of the cluster (transitive overlap).
    let maxEnd = order[i].end;
    let j = i + 1;
    while (j < n && order[j].start < maxEnd) {
      if (order[j].end > maxEnd) maxEnd = order[j].end;
      j++;
    }

    // Greedy color the cluster: pick the smallest column not used by any
    // earlier cluster member that still overlaps this block.
    const cols: number[] = new Array(j - i);
    let maxCol = 0;
    for (let k = i; k < j; k++) {
      const ks = order[k].start;
      const ke = order[k].end;
      const used = new Set<number>();
      for (let m = i; m < k; m++) {
        if (order[m].start < ke && ks < order[m].end) used.add(cols[m - i]);
      }
      let c = 0;
      while (used.has(c)) c++;
      cols[k - i] = c;
      if (c > maxCol) maxCol = c;
    }

    const total = maxCol + 1;
    for (let k = i; k < j; k++) {
      out[order[k].idx] = { col: cols[k - i], cols: total };
    }
    i = j;
  }
  return out;
}
