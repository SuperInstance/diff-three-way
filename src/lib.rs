//! Three-way diff algorithm: compare two versions against a common ancestor.
//!
//! Produces a [`ThreeWayDiff`] with categorized regions.

/// A three-way diff result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeWayDiff {
    pub regions: Vec<DiffRegion>,
}

/// A region in a three-way diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRegion {
    pub line_range: (usize, usize),
    pub kind: RegionKind,
}

/// Classification of a diff region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionKind {
    /// Same in all three versions.
    Unchanged(String),
    /// Changed only in "ours".
    OursOnly { base: String, ours: String },
    /// Changed only in "theirs".
    TheirsOnly { base: String, theirs: String },
    /// Changed in both to the same thing (clean merge).
    BothSame { base: String, merged: String },
    /// Changed in both differently (conflict).
    Conflict { base: String, ours: String, theirs: String },
}

/// Compute a three-way diff of base, ours, and theirs.
pub fn three_way_diff(base: &str, ours: &str, theirs: &str) -> ThreeWayDiff {
    let base_lines: Vec<&str> = base.lines().collect();
    let ours_lines: Vec<&str> = ours.lines().collect();
    let theirs_lines: Vec<&str> = theirs.lines().collect();

    let diff_base_ours = lcs_pairs(&base_lines, &ours_lines);
    let diff_base_theirs = lcs_pairs(&base_lines, &theirs_lines);

    let mut regions = Vec::new();
    let mut line_num = 1usize;

    // Walk base lines, classify each region
    let mut oi = 0usize;
    let mut ti = 0usize;

    for (bi, base_line) in base_lines.iter().enumerate() {
        let ours_changed = oi < diff_base_ours.len()
            && diff_base_ours[oi].0 == bi
            && diff_base_ours[oi].1 != *base_line;
        let theirs_changed = ti < diff_base_theirs.len()
            && diff_base_theirs[ti].0 == bi
            && diff_base_theirs[ti].1 != *base_line;

        let ours_val = if oi < diff_base_ours.len() && diff_base_ours[oi].0 == bi {
            let v = diff_base_ours[oi].1.to_string();
            oi += 1;
            v
        } else {
            base_line.to_string()
        };

        let theirs_val = if ti < diff_base_theirs.len() && diff_base_theirs[ti].0 == bi {
            let v = diff_base_theirs[ti].1.to_string();
            ti += 1;
            v
        } else {
            base_line.to_string()
        };

        let kind = if !ours_changed && !theirs_changed {
            RegionKind::Unchanged(base_line.to_string())
        } else if ours_changed && !theirs_changed {
            RegionKind::OursOnly { base: base_line.to_string(), ours: ours_val }
        } else if !ours_changed && theirs_changed {
            RegionKind::TheirsOnly { base: base_line.to_string(), theirs: theirs_val }
        } else if ours_val == theirs_val {
            RegionKind::BothSame { base: base_line.to_string(), merged: ours_val }
        } else {
            RegionKind::Conflict { base: base_line.to_string(), ours: ours_val, theirs: theirs_val }
        };

        regions.push(DiffRegion { line_range: (line_num, line_num), kind });
        line_num += 1;
    }

    // Handle extra lines in ours or theirs beyond base
    while oi < diff_base_ours.len() || ti < diff_base_theirs.len() {
        if oi < diff_base_ours.len() {
            regions.push(DiffRegion {
                line_range: (line_num, line_num),
                kind: RegionKind::OursOnly { base: String::new(), ours: diff_base_ours[oi].1.to_string() },
            });
            oi += 1;
            line_num += 1;
        }
        if ti < diff_base_theirs.len() {
            regions.push(DiffRegion {
                line_range: (line_num, line_num),
                kind: RegionKind::TheirsOnly { base: String::new(), theirs: diff_base_theirs[ti].1.to_string() },
            });
            ti += 1;
            line_num += 1;
        }
    }

    ThreeWayDiff { regions }
}

impl ThreeWayDiff {
    /// Count conflicts.
    pub fn conflict_count(&self) -> usize {
        self.regions.iter().filter(|r| matches!(r.kind, RegionKind::Conflict { .. })).count()
    }

    /// Check if merge is clean (no conflicts).
    pub fn is_clean(&self) -> bool {
        self.conflict_count() == 0
    }

    /// Generate a merged result, applying non-conflicting changes.
    /// Returns `None` if there are conflicts.
    pub fn try_merge(&self) -> Option<String> {
        if !self.is_clean() {
            return None;
        }
        let mut lines = Vec::new();
        for region in &self.regions {
            match &region.kind {
                RegionKind::Unchanged(s) => lines.push(s.clone()),
                RegionKind::OursOnly { ours, .. } => lines.push(ours.clone()),
                RegionKind::TheirsOnly { theirs, .. } => lines.push(theirs.clone()),
                RegionKind::BothSame { merged, .. } => lines.push(merged.clone()),
                RegionKind::Conflict { .. } => unreachable!(),
            }
        }
        Some(lines.join("\n"))
    }

    /// Render as a side-by-side comparison string.
    pub fn to_side_by_side(&self) -> String {
        let mut out = String::new();
        for region in &self.regions {
            let tag = match &region.kind {
                RegionKind::Unchanged(_) => "   ",
                RegionKind::OursOnly { .. } => "  O",
                RegionKind::TheirsOnly { .. } => " T ",
                RegionKind::BothSame { .. } => " B ",
                RegionKind::Conflict { .. } => " C ",
            };
            let content = match &region.kind {
                RegionKind::Unchanged(s) => s.clone(),
                RegionKind::OursOnly { ours, .. } => ours.clone(),
                RegionKind::TheirsOnly { theirs, .. } => theirs.clone(),
                RegionKind::BothSame { merged, .. } => merged.clone(),
                RegionKind::Conflict { ours, theirs, .. } => format!("OURS: {ours} | THEIRS: {theirs}"),
            };
            out.push_str(&format!("{tag} | {content}\n"));
        }
        out
    }
}

/// Returns pairs of (base_idx, target_line) for matched lines, with mismatches.
fn lcs_pairs<'a>(base: &[&'a str], target: &[&'a str]) -> Vec<(usize, &'a str)> {
    let lcs = longest_common_subsequence(base, target);
    let mut pairs = Vec::new();
    let mut bi = 0usize;
    let mut ti = 0usize;
    let mut li = 0usize;

    while bi < base.len() || ti < target.len() {
        if li < lcs.len() && bi < base.len() && ti < target.len()
            && base[bi] == lcs[li] && target[ti] == lcs[li]
        {
            pairs.push((bi, target[ti]));
            bi += 1; ti += 1; li += 1;
        } else if bi < base.len() && (li >= lcs.len() || base[bi] != lcs[li]) {
            pairs.push((bi, base[bi]));
            bi += 1;
        } else if ti < target.len() {
            pairs.push((bi.saturating_sub(1), target[ti]));
            ti += 1;
        } else {
            break;
        }
    }
    pairs
}

fn longest_common_subsequence<'a>(a: &[&'a str], b: &[&'a str]) -> Vec<&'a str> {
    let m = a.len();
    let n = b.len();
    if m > 500 || n > 500 {
        return a.iter().filter(|l| b.contains(l)).cloned().collect();
    }
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push(a[i - 1]);
            i -= 1; j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_changes() {
        let diff = three_way_diff("a\nb\nc", "a\nb\nc", "a\nb\nc");
        assert!(diff.is_clean());
        assert_eq!(diff.regions.len(), 3);
    }

    #[test]
    fn test_ours_only_change() {
        let diff = three_way_diff("a\nb\nc", "a\nX\nc", "a\nb\nc");
        assert!(diff.is_clean());
        let ours_changes: Vec<_> = diff.regions.iter()
            .filter(|r| matches!(r.kind, RegionKind::OursOnly { .. }))
            .collect();
        assert!(ours_changes.len() >= 1, "expected at least 1 ours change, got regions: {:?", diff.regions);
    }

    #[test]
    fn test_conflict() {
        let diff = three_way_diff("a\nb\nc", "a\nX\nc", "a\nY\nc");
        assert!(diff.conflict_count() >= 1, "expected at least 1 conflict, got {}", diff.conflict_count());
    }
}
