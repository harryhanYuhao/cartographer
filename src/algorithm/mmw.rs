//! Lower bound on treewidth: the **minor-min-width** algorithm (Figure 1 of
//! Gogate & Dechter, UAI 2004).
//!
//! Based on the theorem `tw(G) >= tw(minor of G)` (Lemma 2.9). The algorithm
//! repeatedly contracts the edge between a minimum-degree vertex and the
//! lowest-degree of its neighbors, recording the maximum degree seen. This
//! yields a valid lower bound that empirically dominates the older MCSLB.

use crate::graph::Graph;
use petgraph::graph::NodeIndex;

/// Compute a lower bound on the treewidth of `g` via minor-min-width.
///
/// The input graph is not mutated (a clone is searched).
pub fn minor_min_width(g: &Graph) -> usize {
    if g.alive_count() == 0 {
        return 0;
    }
    let mut work = g.clone();
    let mut lb = 0usize;

    while work.alive_count() > 0 {
        // (a) Pick a minimum-degree alive vertex v.
        let (v, deg_v) = match pick_min_degree(&work) {
            Some(x) => x,
            None => break, // no alive vertices left (shouldn't happen)
        };
        // (b) lb = max(lb, degree(v))  -- measured *before* contraction.
        if deg_v > lb {
            lb = deg_v;
        }
        // (a, cont.) Contract edge {v, u} where u is the min-degree neighbor of v.
        let nbrs = work.alive_neighbors(v);
        if nbrs.is_empty() {
            // Isolated vertex: nothing to contract; just drop it.
            work.mark_dead(v);
            continue;
        }
        let u = nbrs
            .iter()
            .copied()
            .min_by_key(|&n| (work.degree(n), n.index()))
            .expect("non-empty neighbor list");
        // Keep u, drop v (choice is arbitrary; we keep the lower-index endpoint
        // for determinism via the tie-break above on `degree` then `index`).
        work.contract(u, v);
    }

    lb
}

/// Pick the alive vertex of minimum degree, breaking ties by lowest index.
fn pick_min_degree(g: &Graph) -> Option<(NodeIndex, usize)> {
    let mut best: Option<(NodeIndex, usize)> = None;
    for v in g.alive_vertices() {
        let d = g.degree(v);
        match best {
            Some((_, bd)) if d >= bd => {}
            _ => best = Some((v, d)),
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(n: usize) -> Graph {
        Graph::from_edges((0..n - 1).map(|i| (i, i + 1)).collect::<Vec<_>>())
    }

    #[test]
    fn mmw_path_lower_bound_le_one() {
        // A path has treewidth 1, so MMW must be <= 1.
        for n in [2usize, 3, 4, 5, 10] {
            let g = path(n);
            let lb = minor_min_width(&g);
            assert!(lb <= 1, "path of {n} vertices: MMW={lb} should be <= 1");
        }
    }

    #[test]
    fn mmw_clique() {
        // K4 has treewidth 3.
        let g = Graph::from_edges([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]);
        let lb = minor_min_width(&g);
        assert!((1..=3).contains(&lb), "K4: MMW={lb} should be in [1,3]");
    }

    #[test]
    fn mmw_empty_graph() {
        let g = Graph::with_capacity(0);
        assert_eq!(minor_min_width(&g), 0);
    }

    #[test]
    fn mmw_isolated_vertex() {
        let mut g = Graph::with_capacity(3); // 3 isolated vertices
        let _ = &mut g;
        assert_eq!(minor_min_width(&g), 0);
    }
}
