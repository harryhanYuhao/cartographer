//! Upper bound on treewidth: the **min-fill** heuristic (§3 of Gogate &
//! Dechter, UAI 2004).
//!
//! Repeatedly eliminate the vertex that adds the fewest fill-in edges,
//! recording the maximum degree-at-elimination. The paper uses a randomized
//! variant (best of 100 runs); this baseline does a **single deterministic
//! pass** with ties broken by lowest vertex index.

use crate::graph::Graph;
use petgraph::graph::NodeIndex;

/// Run min-fill once deterministically. Returns `(upper_bound, order)` where
/// `order` is the perfect elimination ordering constructed by the heuristic
/// (the first vertex eliminated is `order[0]`).
pub fn min_fill(g: &Graph) -> (usize, Vec<NodeIndex>) {
    let mut work = g.clone();
    let mut order = Vec::with_capacity(g.alive_count());
    let mut ub = 0usize;

    while work.alive_count() > 0 {
        // Pick the alive vertex minimizing fill-in count, ties -> lowest index.
        let v = match pick_min_fill(&work) {
            Some(x) => x,
            None => break,
        };
        let d = work.elim(v);
        if d > ub {
            ub = d;
        }
        order.push(v);
    }

    (ub, order)
}

fn pick_min_fill(g: &Graph) -> Option<NodeIndex> {
    let mut best: Option<(NodeIndex, usize)> = None;
    for v in g.alive_vertices() {
        let f = g.fill_in_count(v);
        match best {
            Some((_, bf)) if f >= bf => {}
            _ => best = Some((v, f)),
        }
    }
    best.map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_fill_path_is_1() {
        // Path of 5 vertices: treewidth 1, min-fill should find 1.
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)]);
        let (ub, order) = min_fill(&g);
        assert_eq!(ub, 1);
        assert_eq!(order.len(), 5);
    }

    #[test]
    fn min_fill_clique() {
        // K4: treewidth 3.
        let g = Graph::from_edges([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        let (ub, order) = min_fill(&g);
        assert_eq!(ub, 3);
        assert_eq!(order.len(), 4);
    }

    #[test]
    fn min_fill_cycle_5_is_2() {
        // C5: treewidth 2.
        let g = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)]);
        let (ub, _) = min_fill(&g);
        assert_eq!(ub, 2);
    }

    #[test]
    fn min_fill_empty() {
        let g = Graph::with_capacity(0);
        let (ub, order) = min_fill(&g);
        assert_eq!(ub, 0);
        assert!(order.is_empty());
    }
}
