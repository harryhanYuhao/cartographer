//! K3 edge removal on degree 2 vertices: a rewrite rule for graph-like ZX-diagrams.
//! (Implement in function k3_remove)
//!
//! The rule targets an alive vertex `v` whose neighbourhood is exactly two
//! distinct neighbours `a`, `b` such that `v`, `a`, `b` form a triangle (K3)
//! of Hadamard edges, all three vertices being Z spiders of arbitrary phase.
//! The rewrite then
//!
//! - leaves the two H edges `{v, a}` and `{v, b}` untouched,
//! - deletes the H edge `{a, b}`,
//! - raises the phases of `a` and `b` by one step (`s*pi/4`, mode 8),
//! - joins a fresh `X(7)` spider to `v` alone, by a normal edge.
//!
//!
//! K3 edge removal hadamard 2:
//! is similar t K3 edge removal degree 2, but it targets a vertex `v` whose
//! order that may be greater than 2, but contain exactly two distinct neighbours `a`, `b`
//! by H edge. (v, a), (v, b) are connected by H edge.
//! Moreover, (a, b) are also connected by H edge.
//! all of a, b, v shall be Z spider of any phase.
//!
//! The rewrite then is
//!
//! Let v be of type z(s)
//!
//! unfuse v into 3 valid vertices u == w == x == v such that
//! u, of type Z(0), is connected to all of normal neighbours of
//! v except a, b via normal edge;
//! w, of type Z(s), is connected to u and x via normal edge;
//! x, of type X(7), is connected to w and v via normal edge;
//! v is connected a and b via H edge, and v become type Z(0)
//! the phases of a and b are raised by one step (s*pi/4, mode 8).

use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};
use crate::operation::utils::h_connected;

fn valid_k3_vertex_degree2(g: &Graph, v: NodeIndex) -> bool {
    if !g.is_alive(v) {
        return false;
    }
    if !matches!(g.label(v), VColor::Z(_)) {
        return false;
    }
    let nbrs = g.alive_neighbors(v);
    if nbrs.len() != 2 {
        return false;
    }
    let (a, b) = (nbrs[0], nbrs[1]);
    if !matches!(g.label(a), VColor::Z(_)) || !matches!(g.label(b), VColor::Z(_)) {
        return false;
    }
    // {v, a, b} must be a triangle of H edges.
    h_connected(g, v, a) && h_connected(g, v, b) && h_connected(g, a, b)
}

fn has_valid_k3_vertex_degree2(g: &Graph) -> Option<NodeIndex> {
    g.alive_vertices().find(|&v| valid_k3_vertex_degree2(g, v))
}

/// Remove an H edge from the first applicable K3 of Z spiders, if any.
pub fn k3_remove_edge_degree2_on_vertex(g: &Graph, v: NodeIndex) -> Graph {
    if !valid_k3_vertex_degree2(g, v) {
        return g.clone();
    }
    let nbrs = g.alive_neighbors(v);
    let (a, b) = (nbrs[0], nbrs[1]);

    let mut out = g.clone();
    // Delete the H edge between the neighbours (one parallel copy, per
    // Graph::remove_edge semantics).
    out.remove_edge(a, b);
    // Neighbours gain one phase step (s*pi/4, wrapping at mode 8).
    for n in [a, b] {
        if let VColor::Z(s) = out.label(n) {
            out.set_color(n, VColor::Z((s + 1) % 8));
        }
    }
    // Attach the fresh X(7) spider to v via a normal edge.
    let x = out.add_vertex_with(VColor::X(7));
    out.add_edge_c(v, x, EColor::NC);
    out
}

/// Apply K3_remove_edge_degree_2_on_vertex repeatedly until no more applicable vertices remain.
pub fn k3_remove(g: &Graph) -> Graph {
    let mut tmp = g.clone();
    loop {
        match has_valid_k3_vertex_degree2(&tmp) {
            Some(v) => tmp = k3_remove_edge_degree2_on_vertex(&tmp, v),
            None => break,
        }
    }
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Z(0)-spider triangle 0-1-2 with all three edges Hadamard.
    fn h_triangle() -> Graph {
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(0));
        let a = g.add_vertex_with(VColor::Z(0));
        let b = g.add_vertex_with(VColor::Z(0));
        for (p, q) in [(v, a), (v, b), (a, b)] {
            g.add_edge_c(p, q, EColor::H);
        }
        g
    }

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
    fn applies_to_h_triangle_of_z_spiders() {
        let before = h_triangle();
        let v = NodeIndex::new(0);
        let out = k3_remove_edge_degree2_on_vertex(&before, v);

        // One new vertex: the X(7) leaf.
        assert_eq!(out.node_count(), 4);
        assert_eq!(out.alive_count(), 4);
        let x = NodeIndex::new(3);
        assert_eq!(out.label(x), VColor::X(7));
        assert_eq!(out.degree(x), 1);

        // Topology + colors: H edges at v kept, H edge a-b gone, x attached
        // to v by a normal edge.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![(0, 1, EColor::H), (0, 2, EColor::H), (0, 3, EColor::NC),]
        );

        // Neighbour phases bumped 0 -> 1; other phases arbitrary but intact.
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(1));

        // Input graph is left untouched.
        assert_eq!(
            sorted_colored_edges(&before),
            vec![(0, 1, EColor::H), (0, 2, EColor::H), (1, 2, EColor::H)]
        );
        for i in 0..3 {
            assert_eq!(before.label(NodeIndex::new(i)), VColor::Z(0));
        }
    }

    #[test]
    fn neighbour_phases_wrap_at_8_and_v_is_untouched() {
        let mut g = h_triangle();
        g.set_color(NodeIndex::new(0), VColor::Z(3));
        g.set_color(NodeIndex::new(1), VColor::Z(7));
        g.set_color(NodeIndex::new(2), VColor::Z(5));

        let out = k3_remove_edge_degree2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(3)); // v keeps phase
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(0)); // 7 + 1 wraps
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(6));
    }

    #[test]
    fn rejects_vertex_of_wrong_degree() {
        // A K3 is symmetric: disabling one corner would merely move the
        // rewrite to another, so give all three a third H neighbour. Then no
        // triangle corner has degree 2 and nothing applies.
        let mut g = h_triangle();
        for target in 0..3 {
            let leaf = g.add_vertex_with(VColor::Z(0));
            g.add_edge_c(NodeIndex::new(target), leaf, EColor::H);
        }

        let out = k3_remove_edge_degree2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), g.node_count());
        assert_eq!(out.edge_count(), g.edge_count());
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&g));
    }

    #[test]
    fn fixpoint_rewrites_every_applicable_triangle() {
        // Two disjoint triangles at nodes {0,1,2} and {3,4,5}: the fixpoint
        // must consume both, appending one X(7) leaf per site (6 and 7).
        let mut g = h_triangle();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(3, 4), (3, 5), (4, 5)] {
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::H);
        }

        let out = k3_remove(&g);
        assert_eq!(out.node_count(), 8);
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 6, EColor::NC),
                (3, 4, EColor::H),
                (3, 5, EColor::H),
                (3, 7, EColor::NC),
            ]
        );
        assert_eq!(out.label(NodeIndex::new(6)), VColor::X(7));
        assert_eq!(out.label(NodeIndex::new(7)), VColor::X(7));
    }

    #[test]
    fn fixpoint_is_idempotent() {
        let mut g = h_triangle();
        let leaf = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(1), leaf, EColor::H);

        let once = k3_remove(&g);
        let twice = k3_remove(&once);
        assert_eq!(sorted_colored_edges(&once), sorted_colored_edges(&twice));
        assert_eq!(once.node_count(), twice.node_count());
    }

    #[test]
    fn rejects_triangle_without_all_h_edges() {
        // Triangle present but every edge is plain (NC), not H.
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(0, 1), (0, 2), (1, 2)] {
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::NC);
        }

        let out = k3_remove_edge_degree2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 3);
        assert_eq!(out.to_graph3(), g.to_graph3());
    }

    #[test]
    fn rejects_without_closed_triangle() {
        // Path 0-1-2: no diagonal edge, so not a K3.
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(0, 1), (0, 2)] {
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::H);
        }

        let out = k3_remove_edge_degree2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 3);
        assert_eq!(out.to_graph3(), g.to_graph3());
    }

    #[test]
    fn rejects_non_z_vertices() {
        let mut g = h_triangle();
        g.set_color(NodeIndex::new(1), VColor::X(0));

        let out = k3_remove_edge_degree2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 3);
        assert_eq!(out.to_graph3(), g.to_graph3());
    }
}
