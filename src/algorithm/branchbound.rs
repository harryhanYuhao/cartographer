//! The base branch-and-bound search (Figure 4 of Gogate & Dechter, UAI 2004).
//!
//! States are tuples `(G_s, x_s)`: the graph `G_s` after eliminating the
//! vertices in partial ordering `x_s` from the original `G`. For each state we
//! maintain:
//!   - `g(s)` = width of the ordering so far (max degree-at-elimination),
//!   - `h(s)` = lower bound on tw(G_s) via minor-min-width,
//!   - `f(s)` = max(g(s), h(s)).
//!
//! The upper bound `ub` is initialized to the min-fill result. We prune a
//! child whenever `f(s') >= ub`. This is the *minimal baseline* per the paper's
//! Figure 4: no simplicial reduction, no neighbor-only branching, no
//! isomorphism pruning, no edge addition.

use crate::algorithm::{minfill, mmw};
use crate::graph::Graph;
use petgraph::graph::NodeIndex;

/// Result of the QuickBB treewidth search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BbResult {
    /// The treewidth of the input graph (exact, when the search completes).
    pub treewidth: usize,
    /// A perfect elimination ordering achieving `treewidth`. The first
    /// eliminated vertex is `order[0]`.
    pub order: Vec<NodeIndex>,
    /// `true` when the search proved optimality (always true for a completed
    /// base search; the field exists so an anytime variant can return `false`).
    pub optimal: bool,
}

/// Compute the treewidth of `g` and an optimal elimination ordering via the
/// base branch-and-bound.
///
/// Short-circuit: if the min-fill upper bound equals the minor-min-width lower
/// bound, that value is the treewidth and no search is needed.
pub fn treewidth(g: &Graph) -> BbResult {
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

    bb(g, 0, &mut path, &mut best);

    BbResult {
        treewidth: best.0,
        order: best.1,
        optimal: true,
    }
}

/// Recursive branch-and-bound. `g` is the current graph, `g_s` is g(s), `path`
/// is the partial elimination ordering `x_s`, `best` is the incumbent.
fn bb(g: &Graph, g_s: usize, path: &mut Vec<NodeIndex>, best: &mut (usize, Vec<NodeIndex>)) {
    // Step 1: if fewer than 2 vertices remain, this path is a complete ordering.
    if g.alive_count() < 2 {
        // f(s) = max(g(s), h(s)); with 0-1 vertices h(s) = 0, so f(s) = g(s).
        if g_s < best.0 {
            best.0 = g_s;
            best.1 = path.clone();
        }
        return;
    }

    // Step 2: branch on every alive vertex.
    // Snapshot the alive vertices first because we will clone `g` per child.
    let candidates: Vec<NodeIndex> = g.alive_vertices().collect();

    for v in candidates {
        let d = g.degree(v);
        let g_child = g_s.max(d);

        // Construct the child graph by cloning and eliminating v.
        let mut gc = g.clone();
        gc.elim(v);

        let h = mmw::minor_min_width(&gc);
        let f = g_child.max(h);

        // Prune: only descend if f < ub.
        if f < best.0 {
            path.push(v);
            bb(&gc, g_child, path, best);
            path.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid3x3() -> Graph {
        // 3x3 grid (nodes 0..8 laid out row-major), treewidth 3.
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

    #[test]
    fn tw_clique_k4() {
        let g = Graph::from_edges([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 3);
        assert_eq!(r.order.len(), 4);
        assert!(r.optimal);
    }

    #[test]
    fn tw_path_p5() {
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)]);
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 1);
        assert_eq!(r.order.len(), 5);
    }

    #[test]
    fn tw_cycle_c5() {
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]);
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 2);
    }

    #[test]
    fn tw_grid3x3() {
        let g = grid3x3();
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 3);
    }

    #[test]
    fn tw_tree_is_one() {
        // A binary tree of depth 3 (15 nodes).
        let mut edges = Vec::new();
        for i in 0..7 {
            edges.push((i, 2 * i + 1));
            edges.push((i, 2 * i + 2));
        }
        let g = Graph::from_edges(edges);
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 1);
    }

    #[test]
    fn tw_star() {
        // Star on 6 vertices: tw = 1.
        let g = star(6);
        let r = treewidth(&g);
        assert_eq!(r.treewidth, 1);
    }

    #[test]
    fn tw_single_and_empty() {
        let empty = Graph::with_capacity(0);
        let r = treewidth(&empty);
        assert_eq!(r.treewidth, 0);

        let single = Graph::with_capacity(1);
        let r = treewidth(&single);
        assert_eq!(r.treewidth, 0);
        assert_eq!(r.order.len(), 1);
    }

    #[test]
    fn determinism() {
        let g = grid3x3();
        let r1 = treewidth(&g);
        let r2 = treewidth(&g);
        assert_eq!(r1, r2);
    }

    #[test]
    fn lower_bound_respected() {
        // For a non-trivial graph, MMW <= actual treewidth.
        let g = grid3x3();
        let r = treewidth(&g);
        let lb = mmw::minor_min_width(&g);
        assert!(lb <= r.treewidth);
    }

    #[test]
    fn order_is_permutation() {
        let g = grid3x3();
        let r = treewidth(&g);
        let mut sorted: Vec<usize> = r.order.iter().map(|n| n.index()).collect();
        sorted.sort();
        assert_eq!(sorted, (0..9).collect::<Vec<_>>());
    }
}
