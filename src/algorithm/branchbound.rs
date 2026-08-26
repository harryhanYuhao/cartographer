//! The QuickBB branch-and-bound search for treewidth.
//!
//! Implements the complete algorithm of Gogate & Dechter, *A Complete
//! Anytime Algorithm for Treewidth* (UAI 2004): the base branch-and-bound
//! (Figure 4) plus the enhancements of §5 and §6:
//!
//!   - **Graph reduction** (§5.1): simplicial and almost-simplicial vertices
//!     are eliminated without branching.
//!   - **Neighbour-only branching** (§5.2, Lemma 5.3): at each state we only
//!     branch on vertices that are non-neighbours (in the *original* graph)
//!     of the last ordered vertex.
//!   - **Edge addition** (§5.3, Theorem 5.4): two vertices sharing more than
//!     `ub` common neighbours are connected.
//!   - **State dominance** (§6, Theorems 6.1/6.3): a state is pruned when the
//!     same graph has already been explored with no larger width.
//!   - **Fill-in dominance** (§6, Theorem 6.4): a candidate whose fill-in
//!     edge set contains another candidate's is pruned.
//!
//! Theorem 6.2 (neighbourhood-invariance pruning) is not implemented.
//!
//! States are tuples `(G_s, x_s)`: the graph `G_s` after eliminating the
//! vertices of partial ordering `x_s` from the original `G`. For each state
//! we maintain:
//!   - `g(s)` = width of the ordering so far (max degree-at-elimination),
//!   - `h(s)` = lower bound on tw(G_s) via minor-min-width,
//!   - `f(s)` = max(g(s), h(s)).
//!
//! The upper bound `ub` is initialised to the min-fill result and is pruned
//! as better orderings are found. A child is pruned whenever
//! `f(s') >= ub`.

use crate::algorithm::{minfill, mmw};
use crate::graph::Graph;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// A collision-free key for a state's graph: its alive vertex set plus the
/// edges among alive vertices. Dead vertices and their incident edges are
/// irrelevant to the remainder of the search.
type GraphKey = (Vec<usize>, Vec<(usize, usize)>);

/// Compute the canonical key of a graph state (alive vertices + alive edges).
fn graph_key(g: &Graph) -> GraphKey {
    let mut alive: Vec<usize> = g.alive_vertices().map(|n| n.index()).collect();
    alive.sort_unstable();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..alive.len() {
        for j in (i + 1)..alive.len() {
            if g.has_edge(NodeIndex::new(alive[i]), NodeIndex::new(alive[j])) {
                edges.push((alive[i], alive[j]));
            }
        }
    }
    (alive, edges)
}

/// Result of the QuickBB treewidth search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BbResult {
    /// The treewidth of the input graph (exact, when the search completes).
    pub treewidth: usize,
    /// A perfect elimination ordering achieving `treewidth`. The first
    /// eliminated vertex is `order[0]`.
    pub order: Vec<NodeIndex>,
    /// `true` when the search proved optimality. The base QuickBB search is
    /// complete, so this is always `true`; the field exists so an anytime
    /// variant can return `false`.
    pub optimal: bool,
}

/// Convenience wrapper returning just the treewidth.
pub fn bb_tw(g: &Graph) -> usize {
    bb(g).treewidth
}

/// Compute the treewidth of `g` and an optimal elimination ordering via the
/// QuickBB branch-and-bound.
///
/// Short-circuit: if the min-fill upper bound equals the minor-min-width lower
/// bound, that value is the treewidth and no search is needed.
pub fn bb(g: &Graph) -> BbResult {
    if g.alive_count() < 2 {
        // 0 or 1 vertex: treewidth 0, order is the (single) vertex.
        let order: Vec<NodeIndex> = g.alive_vertices().collect();
        return BbResult {
            treewidth: 0,
            order,
            optimal: true,
        };
    }

    let (ub, ub_order) = minfill::min_fill(g);
    let lb = mmw::minor_min_width(g);

    if lb == ub {
        return BbResult {
            treewidth: ub,
            order: ub_order,
            optimal: true,
        };
    }

    // Run the search. `best` holds (width, ordering) for the incumbent.
    let mut best: (usize, Vec<NodeIndex>) = (ub, ub_order);
    let mut path: Vec<NodeIndex> = Vec::with_capacity(g.alive_count());

    // State-dominance memo (§6.1/§6.3): map from graph key to the smallest
    // `g(s)` with which that graph has been (or is being) explored.
    let mut seen: HashMap<GraphKey, usize> = HashMap::new();

    // `g` is never mutated; `work` is the mutable per-state graph.
    let mut work = g.clone();
    bb_helper(&mut work, g, 0, &mut path, &mut best, &mut seen);

    BbResult {
        treewidth: best.0,
        order: best.1,
        optimal: true,
    }
}

/// Recursive branch-and-bound.
///
/// * `g` — the current graph `G_s` (owned by this call; mutated in place by
///   the reduction and edge-addition rules, and cloned for each child).
/// * `original` — the immutable input graph, used only for the neighbour-only
///   branching test of §5.2 (which needs *original*, not fill-in, adjacency).
/// * `g_val` — `g(s)`, the width of the partial ordering so far.
/// * `path` — the partial elimination ordering `x_s`.
/// * `best` — the incumbent `(width, ordering)`.
/// * `seen` — state-dominance memo: graph key → smallest `g` seen.
fn bb_helper(
    g: &mut Graph,
    original: &Graph,
    mut g_val: usize,
    path: &mut Vec<NodeIndex>,
    best: &mut (usize, Vec<NodeIndex>),
    seen: &mut HashMap<GraphKey, usize>,
) {
    // §6.1/§6.3: if this exact graph was already (or is being) explored with a
    // width no larger than ours, every completion we could find here is
    // dominated, so prune. Otherwise record our (better) width.
    let key = graph_key(g);
    match seen.get(&key) {
        Some(&prev) if prev <= g_val => return,
        _ => {
            seen.insert(key, g_val);
        }
    }

    // §5.1 + §5.3: reduce and add edges until a fixed point.
    reduce_and_add_edges(g, &mut g_val, path, best.0);

    // Base case: fewer than 2 vertices remain, this path is a complete ordering.
    if g.alive_count() < 2 {
        if g_val < best.0 {
            best.0 = g_val;
            best.1 = path.clone();
        }
        return;
    }

    // §5.2: branch only on non-neighbours of the last ordered vertex.
    let candidates = branch_candidates(g, original, path.last().copied());
    // §6 (Theorem 6.4): drop fill-in-dominated candidates.
    let candidates = prune_dominated(g, candidates);

    for v in candidates {
        let d = g.degree(v);
        let g_child = g_val.max(d);

        // Construct the child graph by cloning and eliminating v.
        let mut gc = g.clone();
        gc.elim(v);

        let h = mmw::minor_min_width(&gc);
        let f = g_child.max(h);

        // Prune: only descend if f < ub.
        if f < best.0 {
            path.push(v);
            bb_helper(&mut gc, original, g_child, path, best, seen);
            path.pop();
        }
    }
}

/// Apply the §5.1 graph-reduction rules and the §5.3 edge-addition rule
/// alternately until neither changes the graph.
fn reduce_and_add_edges(g: &mut Graph, g_val: &mut usize, path: &mut Vec<NodeIndex>, ub: usize) {
    loop {
        reduce(g, g_val, path);
        if !add_edges(g, ub) {
            break;
        }
    }
}

/// §5.1: eliminate simplicial vertices, and almost-simplicial vertices whose
/// degree is at most a (recomputed) lower bound, until none remain.
fn reduce(g: &mut Graph, g_val: &mut usize, path: &mut Vec<NodeIndex>) {
    // A cached minor-min-width lower bound on the *current* graph, recomputed
    // lazily because eliminating a vertex changes the graph (and can change
    // the lower bound). `None` means "stale".
    let mut lb: Option<usize> = None;

    loop {
        // Simplicial vertices are safe unconditionally (no lower bound needed).
        if let Some(v) = find_simplicial(g) {
            let d = g.elim(v);
            *g_val = (*g_val).max(d);
            path.push(v);
            lb = None;
            continue;
        }

        // Almost-simplicial vertices are safe only when degree <= tw(G); we
        // approximate tw(G) with a fresh minor-min-width lower bound.
        if lb.is_none() {
            lb = Some(mmw::minor_min_width(g));
        }
        if let Some(v) = find_almost_simplicial(g, lb.expect("lb computed above")) {
            let d = g.elim(v);
            *g_val = (*g_val).max(d);
            path.push(v);
            lb = None;
            continue;
        }

        break;
    }
}

/// Lowest-index simplicial alive vertex, if any.
fn find_simplicial(g: &Graph) -> Option<NodeIndex> {
    g.alive_vertices().find(|&v| g.is_simplicial(v))
}

/// Lowest-index almost-simplicial alive vertex of degree at most `lb`, if any.
fn find_almost_simplicial(g: &Graph, lb: usize) -> Option<NodeIndex> {
    g.alive_vertices()
        .find(|&v| g.degree(v) <= lb && g.is_almost_simplicial(v))
}

/// §5.3 (Theorem 5.4): connect every pair of non-adjacent vertices sharing
/// more than `ub` common neighbours, repeatedly until no such pair remains.
///
/// Returns `true` if at least one edge was added.
fn add_edges(g: &mut Graph, ub: usize) -> bool {
    let mut added = false;

    loop {
        let mut changed = false;
        let vs: Vec<NodeIndex> = g.alive_vertices().collect();
        for i in 0..vs.len() {
            for j in (i + 1)..vs.len() {
                let (a, b) = (vs[i], vs[j]);
                if g.has_edge(a, b) {
                    continue;
                }
                if g.common_neighbors(a, b) >= ub + 1 {
                    g.add_edge(a, b);
                    added = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    added
}

/// §5.2 (Lemma 5.3): the successors of a state are the alive vertices that are
/// non-neighbours — in the *original* graph — of the last ordered vertex.
///
/// If no such vertex exists (the "otherwise" branch of Definition 5.2), every
/// alive vertex is a candidate; this can only happen when the remaining graph
/// is a clique, which the simplicial reduction normally already exhausts.
fn branch_candidates(g: &Graph, original: &Graph, last: Option<NodeIndex>) -> Vec<NodeIndex> {
    let Some(v_s) = last else {
        // Root state: no previous vertex, branch on everything.
        return g.alive_vertices().collect();
    };

    let non_neighbours: Vec<NodeIndex> = g
        .alive_vertices()
        .filter(|&v| !original.has_edge(v_s, v))
        .collect();

    if non_neighbours.is_empty() {
        g.alive_vertices().collect()
    } else {
        non_neighbours
    }
}

/// §6 (Theorem 6.4): a candidate `B` is dominated by candidate `A` when the
/// fill-in edge set of `A` is a subset of that of `B`; the `B` branch is then
/// pruned. Equal fill-in sets are deduplicated by keeping the lower index.
fn prune_dominated(g: &Graph, candidates: Vec<NodeIndex>) -> Vec<NodeIndex> {
    if candidates.len() < 2 {
        return candidates;
    }

    let fills: Vec<Vec<(usize, usize)>> = candidates
        .iter()
        .map(|&v| {
            let mut e: Vec<(usize, usize)> = g
                .fill_in_edges(v)
                .into_iter()
                .map(|(a, b)| (a.index(), b.index()))
                .collect();
            e.sort_unstable();
            e
        })
        .collect();

    let mut keep = Vec::with_capacity(candidates.len());
    for i in 0..candidates.len() {
        let mut dominated = false;
        for j in 0..candidates.len() {
            if i == j {
                continue;
            }
            if fills[j].len() < fills[i].len() {
                if is_subset(&fills[j], &fills[i]) {
                    dominated = true;
                    break;
                }
            } else if fills[j].len() == fills[i].len() && fills[j] == fills[i] && j < i {
                // Identical fill-in sets: keep the lower index deterministically.
                dominated = true;
                break;
            }
        }
        if !dominated {
            keep.push(candidates[i]);
        }
    }

    keep
}

/// Is every element of sorted `a` also present in `b`?
fn is_subset(a: &[(usize, usize)], b: &[(usize, usize)]) -> bool {
    a.iter().all(|x| b.contains(x))
}

#[cfg(test)]
mod tests {
    use super::*;

    // The original Figure 4 baseline (full branching, no enhancements), kept as
    // an independent oracle for differential testing.
    fn bb_baseline(g: &Graph) -> BbResult {
        if g.alive_count() < 2 {
            let order: Vec<NodeIndex> = g.alive_vertices().collect();
            return BbResult { treewidth: 0, order, optimal: true };
        }
        let (ub, ub_order) = minfill::min_fill(g);
        let lb = mmw::minor_min_width(g);
        if lb == ub {
            return BbResult { treewidth: ub, order: ub_order, optimal: true };
        }
        let mut best = (ub, ub_order);
        let mut path = Vec::with_capacity(g.alive_count());
        bb_baseline_helper(g, 0, &mut path, &mut best);
        BbResult { treewidth: best.0, order: best.1, optimal: true }
    }

    fn bb_baseline_helper(g: &Graph, g_s: usize, path: &mut Vec<NodeIndex>, best: &mut (usize, Vec<NodeIndex>)) {
        if g.alive_count() < 2 {
            if g_s < best.0 {
                best.0 = g_s;
                best.1 = path.clone();
            }
            return;
        }
        let candidates: Vec<NodeIndex> = g.alive_vertices().collect();
        for v in candidates {
            let d = g.degree(v);
            let g_child = g_s.max(d);
            let mut gc = g.clone();
            gc.elim(v);
            let h = mmw::minor_min_width(&gc);
            let f = g_child.max(h);
            if f < best.0 {
                path.push(v);
                bb_baseline_helper(&gc, g_child, path, best);
                path.pop();
            }
        }
    }

    fn grid3x3() -> Graph {
        let mut edges = Vec::new();
        for r in 0..3 {
            for c in 0..3 {
                let i = r * 3 + c;
                if c + 1 < 3 {
                    edges.push((i, i + 1));
                }
                if r + 1 < 3 {
                    edges.push((i, i + 3));
                }
            }
        }
        Graph::from_edges(edges)
    }

    fn star(n: usize) -> Graph {
        Graph::from_edges((1..n).map(|i| (0, i)).collect::<Vec<_>>())
    }

    /// Simulate `order` on a fresh copy of `g` and return the elimination
    /// width, i.e. the maximum degree-at-elimination.
    fn elimination_width(g: &Graph, order: &[NodeIndex]) -> usize {
        let mut work = g.clone();
        let mut width = 0;
        for &v in order {
            width = width.max(work.elim(v));
        }
        width
    }

    fn is_permutation(g: &Graph, order: &[NodeIndex]) -> bool {
        let mut sorted: Vec<usize> = order.iter().map(|n| n.index()).collect();
        sorted.sort();
        let mut expected: Vec<usize> = g.alive_vertices().map(|n| n.index()).collect();
        expected.sort();
        sorted == expected
    }

    /// Build every simple graph on `n` labelled vertices.
    fn all_graphs(n: usize) -> Vec<Graph> {
        let max = n.saturating_mul(n.saturating_sub(1)) / 2;
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                pairs.push((i, j));
            }
        }
        let mut out = Vec::with_capacity(1usize << max);
        for mask in 0u64..(1u64 << max) {
            let mut g = Graph::with_capacity(n);
            for (k, &(a, b)) in pairs.iter().enumerate() {
                if (mask >> k) & 1 == 1 {
                    g.add_edge(NodeIndex::new(a), NodeIndex::new(b));
                }
            }
            out.push(g);
        }
        out
    }

    #[test]
    fn tw_clique_k4() {
        let g = Graph::from_edges([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        let r = bb(&g);
        assert_eq!(r.treewidth, 3);
        assert_eq!(r.order.len(), 4);
        assert!(r.optimal);
    }

    #[test]
    fn tw_path_p5() {
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)]);
        let r = bb(&g);
        assert_eq!(r.treewidth, 1);
        assert_eq!(r.order.len(), 5);
    }

    #[test]
    fn tw_cycle_c5() {
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]);
        let r = bb(&g);
        assert_eq!(r.treewidth, 2);
    }

    #[test]
    fn tw_grid3x3() {
        let g = grid3x3();
        let r = bb(&g);
        assert_eq!(r.treewidth, 3);
    }

    #[test]
    fn tw_tree_is_one() {
        let mut edges = Vec::new();
        for i in 0..7 {
            edges.push((i, 2 * i + 1));
            edges.push((i, 2 * i + 2));
        }
        let g = Graph::from_edges(edges);
        let r = bb(&g);
        assert_eq!(r.treewidth, 1);
    }

    #[test]
    fn tw_star() {
        let g = star(6);
        let r = bb(&g);
        assert_eq!(r.treewidth, 1);
    }

    #[test]
    fn tw_single_and_empty() {
        let empty = Graph::with_capacity(0);
        let r = bb(&empty);
        assert_eq!(r.treewidth, 0);

        let single = Graph::with_capacity(1);
        let r = bb(&single);
        assert_eq!(r.treewidth, 0);
        assert_eq!(r.order.len(), 1);
    }

    #[test]
    fn determinism() {
        let g = grid3x3();
        let r1 = bb(&g);
        let r2 = bb(&g);
        assert_eq!(r1, r2);
    }

    #[test]
    fn lower_bound_respected() {
        let g = grid3x3();
        let r = bb(&g);
        let lb = mmw::minor_min_width(&g);
        assert!(lb <= r.treewidth);
    }

    #[test]
    fn order_is_permutation() {
        let g = grid3x3();
        let r = bb(&g);
        assert!(is_permutation(&g, &r.order));
    }

    /// The enhanced search must agree with the naive Figure 4 baseline on
    /// every labelled graph up to 6 vertices, and must return a valid optimal
    /// ordering (a permutation whose elimination width equals the treewidth).
    #[test]
    fn agrees_with_baseline_up_to_6_vertices() {
        for n in 0..=6usize {
            for g in all_graphs(n) {
                let expected = bb_baseline(&g).treewidth;
                let r = bb(&g);
                assert_eq!(
                    r.treewidth, expected,
                    "n={n}: enhanced tw {} != baseline tw {}",
                    r.treewidth, expected
                );
                assert!(r.optimal);
                assert!(is_permutation(&g, &r.order), "n={n}: order not a permutation");
                assert_eq!(
                    elimination_width(&g, &r.order),
                    r.treewidth,
                    "n={n}: witness ordering width != reported treewidth"
                );
            }
        }
    }

    /// Spot-check some small named graphs against the baseline.
    #[test]
    fn agrees_with_baseline_on_named_graphs() {
        use crate::generator::named::{cycle, grid, path};

        for n in 2..=10 {
            let g = path(n);
            assert_eq!(bb(&g).treewidth, bb_baseline(&g).treewidth, "path({n})");
        }
        for n in 3..=9 {
            let g = cycle(n);
            assert_eq!(bb(&g).treewidth, bb_baseline(&g).treewidth, "cycle({n})");
        }
        // The naive baseline is exponential, so keep grids tiny.
        for (r, c) in [(3, 3), (3, 4), (4, 3)] {
            let g = grid(r, c);
            assert_eq!(bb(&g).treewidth, bb_baseline(&g).treewidth, "grid({r}x{c})");
        }
    }
}
