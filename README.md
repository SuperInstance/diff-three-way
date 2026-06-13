# Three-Way Diff

**A three-way diff library** that compares two versions against a common ancestor and categorizes every change into one of five types — unchanged, ours-only, theirs-only, both-same, or conflict — providing the structured foundation for merge tools.

## Why It Matters

Git's merge engine is fundamentally a three-way diff. When you run `git merge feature-branch`, Git doesn't just compare your branch with theirs — it retrieves the merge base (the common ancestor commit) and performs a three-way comparison. This attribution is essential: without the base, you can't tell who made which change.

This library provides the structured diff that merge tools need. Rather than producing a flat text output, it returns typed `RegionKind` variants that classify each line:

- **Unchanged** — Same in base, ours, and theirs
- **OursOnly** — Changed only on our side (auto-accept)
- **TheirsOnly** — Changed only on their side (auto-accept)
- **BothSame** — Both sides made the *same* change (clean merge)
- **Conflict** — Both sides made *different* changes (needs resolution)

This classification enables `try_merge()` (automatic resolution when there are no conflicts) and `to_side_by_side()` (visual comparison for conflict resolution UIs).

## How It Works

The algorithm operates in two stages:

**1. Pairwise LCS diffing:** Two LCS-based diffs are computed — one between base and ours, another between base and theirs. Each diff produces a sequence of (base_index, target_line) pairs, identifying which lines match and which changed.

**2. Synchronized classification:** The algorithm walks the base lines, consulting both diffs simultaneously. For each base line, it checks whether ours changed it and whether theirs changed it, producing one of five classifications:

| Ours Changed? | Theirs Changed? | Classification |
|:---:|:---:|:---|
| No | No | Unchanged |
| Yes | No | OursOnly |
| No | Yes | TheirsOnly |
| Yes | Yes (same result) | BothSame |
| Yes | Yes (different results) | Conflict |

Lines beyond the base length (pure insertions) are handled separately as additions to the appropriate side.

**LCS computation:** The longest common subsequence is computed via standard O(N×M) dynamic programming. For inputs exceeding 500 lines, a simpler intersection-based fallback avoids the quadratic blowup.

## Quick Start

```rust
use diff_three_way::{three_way_diff, RegionKind};

let base = "line 1\nline 2\nline 3";
let ours = "line 1\nLINE TWO\nline 3";      // changed line 2
let theirs = "line 1\nline 2\nLINE THREE";   // changed line 3

let diff = three_way_diff(base, ours, theirs);

// Non-overlapping changes → clean merge
assert!(diff.is_clean());

if let Some(merged) = diff.try_merge() {
    println!("Merged:\n{}", merged);
    // "line 1\nLINE TWO\nLINE THREE"
}

// Conflict example
let diff = three_way_diff("a\nb\nc", "a\nX\nc", "a\nY\nc");
assert!(diff.conflict_count() >= 1);

// Side-by-side view
println!("{}", diff.to_side_by_side());
```

## API

### `three_way_diff(base, ours, theirs) -> ThreeWayDiff`
- Compute three-way diff. O(N×M) for LCS computation

### `ThreeWayDiff`
- `regions: Vec<DiffRegion>` — All diff regions
- `conflict_count() -> usize` — Number of conflicting regions
- `is_clean() -> bool` — True if no conflicts
- `try_merge() -> Option<String>` — Auto-merge if clean, None if conflicts
- `to_side_by_side() -> String` — Visual side-by-side comparison

### `RegionKind` (enum)
- `Unchanged(String)` — Same in all three versions
- `OursOnly { base, ours }` — Changed only on our side
- `TheirsOnly { base, theirs }` — Changed only on their side
- `BothSame { base, merged }` — Both sides made the same change
- `Conflict { base, ours, theirs }` — Both sides changed differently

## Architecture Notes

Part of SuperInstance's text processing toolkit alongside Myers diff, Patience diff, and diff-merge. This library provides the structured diff analysis used by the merge engine for version control operations and collaborative editing.

See the full architecture: [ARCHITECTURE.md](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md)

## License

MIT
