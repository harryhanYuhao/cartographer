//! Hadamard-edge parity normalization: parallel H edges cancel in pairs.
//!
//! `H * H = I`: a pair of vertices joined by an even number of H edges is
//! equivalent to no edge at all, and an odd number of H edges to exactly one.
//!
//! [`normalize_h_parity`] collapses the H edges of one pair to that single
//! equivalent copy; [`normalize_h_parity_total`] does the same for every
//! H-connected pair of distinct vertices in the graph. Parallel NC copies on
//! a mixed pair are re-attached unchanged, and every self-loop is removed
//! outright, whatever its colour.

use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph};

/// Number of H-coloured edges between `a` and `b`.
fn h_edge_count(g: &Graph, a: NodeIndex, b: NodeIndex) -> usize {
    g.edges()
        .filter(|&(s, t, e)| {
            g.edge_color(e) == EColor::H && ((s == a && t == b) || (s == b && t == a))
        })
        .count()
}

/// Are `a` and `b` joined by an odd number of H edges? Even counts are
/// equivalent to no edge at all (`H * H = I`).
pub fn odd_h_connected(g: &Graph, a: NodeIndex, b: NodeIndex) -> bool {
    h_edge_count(g, a, b) % 2 == 1
}

/// Collapse the H edges between `a` and `b` (in place) to their parity: an
/// odd number of copies becomes a single H edge, an even number none.
/// Parallel NC copies (mixed pairs) are re-attached unchanged. A self-loop
/// pair (`a == b`) is removed outright, colour and copies alike.
fn normalize_pair(g: &mut Graph, a: NodeIndex, b: NodeIndex) {
    if a == b {
        while g.edge_multiplicity(a, a) > 0 {
            g.remove_edge(a, a);
        }
        return;
    }
    let (mut nc, mut h) = (0usize, 0usize);
    for (s, t, e) in g.edges() {
        if (s == a && t == b) || (s == b && t == a) {
            if g.edge_color(e) == EColor::NC {
                nc += 1;
            } else {
                h += 1;
            }
        }
    }
    while g.edge_multiplicity(a, b) > 0 {
        g.remove_edge(a, b);
    }
    for _ in 0..nc {
        g.add_edge_c(a, b, EColor::NC);
    }
    if h % 2 == 1 {
        g.add_edge_c(a, b, EColor::H);
    }
}

/// Normalize the H-edge parity between `a` and `b`: an odd number of H
/// copies collapses to a single H edge, an even number to none. Parallel NC
/// copies (mixed pairs) are kept; with `a == b` every self-loop at that
/// vertex is removed; every other edge of the graph is untouched.
pub fn normalize_h_parity(g: &Graph, a: NodeIndex, b: NodeIndex) -> Graph {
    let mut out = g.clone();
    normalize_pair(&mut out, a, b);
    out
}

/// The distinct pairs of alive vertices joined by at least one H edge,
/// ascending; self-loops are excluded.
fn h_pairs(g: &Graph) -> Vec<(NodeIndex, NodeIndex)> {
    let mut pairs: Vec<(NodeIndex, NodeIndex)> = g
        .edges()
        .filter(|&(s, t, e)| g.edge_color(e) == EColor::H && s != t)
        .map(|(s, t, _)| (s.min(t), s.max(t)))
        .collect();
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Apply [`normalize_h_parity`] to every H-connected pair of distinct
/// vertices, and remove every self-loop. Idempotent: a normalized graph is
/// returned unchanged.
pub fn normalize_h_parity_total(g: &Graph) -> Graph {
    let mut out = g.clone();
    // Self-loops are removed outright, colour and copies alike.
    for v in g.alive_vertices() {
        while out.edge_multiplicity(v, v) > 0 {
            out.remove_edge(v, v);
        }
    }
    for (a, b) in h_pairs(g) {
        normalize_pair(&mut out, a, b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::VColor;

    /// Sorted `(u, v, color)` triples of the alive edges, u <= v.
    fn sorted_colored_edges(g: &Graph) -> Vec<(usize, usize, EColor)> {
        let mut out: Vec<_> = g
            .edges()
            .map(|(s, t, e)| {
                let (x, y) = (s.index(), t.index());
                (x.min(y), x.max(y), g.edge_color(e))
            })
            .collect();
        out.sort_by_key(|&(x, y, _)| (x, y));
        out
    }

    #[test]
    fn collapses_odd_to_one_and_even_to_none() {
        // 0-1: 3 H (odd), 1-2: 2 H (even), 0-2: 1 H.
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        for _ in 0..3 {
            g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);
        }
        for _ in 0..2 {
            g.add_edge_c(NodeIndex::new(1), NodeIndex::new(2), EColor::H);
        }
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(2), EColor::H);

        let out = normalize_h_parity_total(&g);
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            1
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            0
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)),
            1
        );

        // Input untouched, and the pass is idempotent.
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 2);
        let twice = normalize_h_parity_total(&out);
        assert_eq!(sorted_colored_edges(&twice), sorted_colored_edges(&out));
    }

    #[test]
    fn keeps_nc_copies_on_mixed_pairs() {
        let mut g = Graph::new();
        g.add_vertex_with(VColor::Z(0));
        g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        for _ in 0..3 {
            g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);
        }

        // 3 H -> 1 H, both NC copies re-attached; same via the pair and the
        // total version.
        let expected = vec![(0, 1, EColor::NC), (0, 1, EColor::NC), (0, 1, EColor::H)];
        assert_eq!(
            sorted_colored_edges(&normalize_h_parity(
                &g,
                NodeIndex::new(0),
                NodeIndex::new(1)
            )),
            expected
        );
        assert_eq!(
            sorted_colored_edges(&normalize_h_parity_total(&g)),
            expected
        );
    }

    #[test]
    fn pair_version_only_touches_its_pair() {
        // Two even-H pairs; normalizing 0-1 leaves 2-3 alone.
        let mut g = Graph::new();
        for _ in 0..4 {
            g.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(0usize, 1usize), (2, 3)] {
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::H);
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::H);
        }

        let out = normalize_h_parity(&g, NodeIndex::new(0), NodeIndex::new(1));
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            0
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(2), NodeIndex::new(3)),
            2
        );
    }

    #[test]
    fn removes_self_loops() {
        // H self-loops x2 (even) and one NC self-loop at 0, another NC
        // self-loop at 1, plus an even-H pair 0-1: the total pass removes
        // every self-loop, whatever its colour, and the pair still collapses.
        let mut g = Graph::new();
        g.add_vertex_with(VColor::Z(0));
        g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::H);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::H);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::NC);
        g.add_edge_c(NodeIndex::new(1), NodeIndex::new(1), EColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);

        let out = normalize_h_parity_total(&g);
        assert_eq!(sorted_colored_edges(&out), vec![]);

        // The pair version with a == b only strips that vertex's self-loops.
        let pair = normalize_h_parity(&g, NodeIndex::new(0), NodeIndex::new(0));
        assert_eq!(
            sorted_colored_edges(&pair),
            vec![(0, 1, EColor::H), (0, 1, EColor::H), (1, 1, EColor::NC),]
        );

        // A graph with neither H pairs nor self-loops is returned unchanged,
        // as is the empty graph.
        let mut nc_only = Graph::new();
        nc_only.add_vertex_with(VColor::Z(3));
        nc_only.add_vertex_with(VColor::Z(0));
        nc_only.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        assert_eq!(
            sorted_colored_edges(&normalize_h_parity_total(&nc_only)),
            sorted_colored_edges(&nc_only)
        );
        assert_eq!(normalize_h_parity_total(&Graph::new()).node_count(), 0);
    }
}
