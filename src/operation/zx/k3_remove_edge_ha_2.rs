//! This file implement the operation
//! K3 edge removal hadamard 2, which
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

use crate::operation::{
    utils::{get_h_neighbour, get_normal_neighbour, h_connected},
    zx::unfuse::unfuse_to_sep_h_edge_in_place,
};
use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};

/// A valid had2 target: an alive Z vertex with exactly two distinct H-edge
/// neighbours `a`, `b` (its total order may be larger via normal edges) that
/// together form an all-H triangle of Z spiders.
fn valid_k3_vertex_had2(g: &Graph, v: NodeIndex) -> bool {
    if !g.is_alive(v) {
        return false;
    }
    if !matches!(g.label(v), VColor::Z(_)) {
        return false;
    }
    let nbrs = get_h_neighbour(g, v);
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

fn has_valid_k3_vertex_had2(g: &Graph) -> Option<NodeIndex> {
    g.alive_vertices().find(|&v| valid_k3_vertex_had2(g, v))
}

/// Apply the K3 edge removal hadamard 2 rewrite at `v`; if `v` is not a valid
/// target, return the graph unchanged.
pub fn k3_remove_edge_had2_on_vertex(g: &Graph, v: NodeIndex) -> Graph {
    let mut out = g.clone();
    k3_remove_edge_had2_in_place(&mut out, v);
    out
}

/// In-place core of [`k3_remove_edge_had2_on_vertex`]: mutates `g` directly
/// instead of cloning at every step, leaving it untouched when `v` is not a
/// valid target.
pub(crate) fn k3_remove_edge_had2_in_place(g: &mut Graph, v: NodeIndex) {
    // Do nothing if not on valid vertex
    if !valid_k3_vertex_had2(g, v) {
        return;
    }
    unfuse_to_sep_h_edge_in_place(g, v);

    let h_nbrs = get_h_neighbour(g, v);
    assert_eq!(h_nbrs.len(), 2);
    let (a, b) = (h_nbrs[0], h_nbrs[1]);

    let normal_nbrs = get_normal_neighbour(g, v);
    assert_eq!(normal_nbrs.len(), 1);
    let w = normal_nbrs[0];

    // Delete the H edge between the neighbours (one parallel copy, per
    // Graph::remove_edge semantics).
    g.remove_edge(a, b);
    // Neighbours gain one phase step (s*pi/4, wrapping at mode 8).
    for n in [a, b] {
        if let VColor::Z(s) = g.label(n) {
            g.set_color(n, VColor::Z((s + 1) % 8));
        }
    }
    g.remove_edge(w, v);
    // Attach the fresh X(7) spider to v via a normal edge.
    let x = g.add_vertex_with(VColor::X(7));
    g.add_edge_c(v, x, EColor::NC);
    g.add_edge_c(w, x, EColor::NC);
}

/// Apply k3_remove_edge_had2_on_vertex repeatedly until no more applicable
/// vertices remain.
pub fn k3_remove_had2(g: &Graph) -> Graph {
    let mut tmp = g.clone();
    while let Some(v) = has_valid_k3_vertex_had2(&tmp) {
        k3_remove_edge_had2_in_place(&mut tmp, v);
    }
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// v=Z(pv) with H triangle a=Z(pa), b=Z(pb) and normal neighbours
    /// r1=Z(pr), r2=X(px). Returns (graph, indices).
    fn had2_site(pv: u8, pa: u8, pb: u8) -> Graph {
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(pv));
        let a = g.add_vertex_with(VColor::Z(pa));
        let b = g.add_vertex_with(VColor::Z(pb));
        let r1 = g.add_vertex_with(VColor::Z(5));
        let r2 = g.add_vertex_with(VColor::X(2));
        for (p, q) in [(v, a), (v, b), (a, b)] {
            g.add_edge_c(p, q, EColor::H);
        }
        g.add_edge_c(v, r1, EColor::NC);
        g.add_edge_c(v, r2, EColor::NC);
        g
    }

    #[test]
    fn rewrites_triangle_with_extra_normal_neighbours() {
        let g = had2_site(3, 1, 0);
        let out = k3_remove_edge_had2_on_vertex(&g, NodeIndex::new(0));

        // unfuse appends u = node 5, w = node 6; the X(7) node x = node 7.
        assert_eq!(out.node_count(), 8);

        // Chain r1,r2 == u(Z0) == w(Z3) == x(X7) == v(Z0); the two H edges at
        // v are kept, the a-b diagonal is gone.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 7, EColor::NC),
                (3, 5, EColor::NC),
                (4, 5, EColor::NC),
                (5, 6, EColor::NC),
                (6, 7, EColor::NC),
            ]
        );

        // Labels: v resets to Z(0), w carries v's old phase 3, x is X(7),
        // neighbours' phases bump by one, everyone else is untouched.
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(2)); // 1 + 1
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(1)); // 0 + 1
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(5));
        assert_eq!(out.label(NodeIndex::new(4)), VColor::X(2));
        assert_eq!(out.label(NodeIndex::new(5)), VColor::Z(0)); // u
        assert_eq!(out.label(NodeIndex::new(6)), VColor::Z(3)); // w
        assert_eq!(out.label(NodeIndex::new(7)), VColor::X(7)); // x

        // Input graph is left untouched.
        assert_eq!(g.node_count(), 5);
        assert_eq!(
            sorted_colored_edges(&g),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 3, EColor::NC),
                (0, 4, EColor::NC),
                (1, 2, EColor::H),
            ]
        );
    }

    #[test]
    fn applies_with_no_normal_neighbours_and_wraps_phases() {
        // v = Z(7) in a bare H triangle: still a valid had2 target (order 2),
        // and a = Z(7) wraps to Z(0) after the bump.
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(7));
        let a = g.add_vertex_with(VColor::Z(7));
        let b = g.add_vertex_with(VColor::Z(5));
        for (p, q) in [(v, a), (v, b), (a, b)] {
            g.add_edge_c(p, q, EColor::H);
        }

        let out = k3_remove_edge_had2_on_vertex(&g, v);
        // u = 3 (only joined to w), w = 4, x = 5.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 5, EColor::NC),
                (3, 4, EColor::NC),
                (4, 5, EColor::NC),
            ]
        );
        assert_eq!(out.label(v), VColor::Z(0));
        assert_eq!(out.label(a), VColor::Z(0)); // 7 + 1 wraps
        assert_eq!(out.label(b), VColor::Z(6));
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(0)); // u
        assert_eq!(out.label(NodeIndex::new(4)), VColor::Z(7)); // w keeps pv
        assert_eq!(out.label(NodeIndex::new(5)), VColor::X(7)); // x
        assert_eq!(out.degree(NodeIndex::new(3)), 1); // u touches only w
    }

    #[test]
    fn returns_unchanged_for_invalid_targets() {
        // Node 0 gains a third H neighbour, so its H-degree is wrong. Nodes 1
        // and 2 stay valid (a K3 is symmetric), so only 0, 3, 4, 5 must come
        // back untouched.
        let mut g = had2_site(0, 0, 0);
        let c = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(0), c, EColor::H);

        for target in [0usize, 3, 4, 5] {
            let out = k3_remove_edge_had2_on_vertex(&g, NodeIndex::new(target));
            assert_eq!(out.node_count(), g.node_count(), "target {target}");
            assert_eq!(
                sorted_colored_edges(&out),
                sorted_colored_edges(&g),
                "target {target}"
            );
        }
    }

    #[test]
    fn returns_unchanged_without_closed_h_triangle() {
        // Drop the a-b diagonal: no K3 anywhere.
        let mut g = had2_site(0, 0, 0);
        g.remove_edge(NodeIndex::new(1), NodeIndex::new(2));

        let out = k3_remove_edge_had2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), g.node_count());
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&g));
    }

    #[test]
    fn returns_unchanged_for_non_z_triangle_member() {
        let mut g = had2_site(0, 0, 0);
        g.set_color(NodeIndex::new(1), VColor::X(0));

        let out = k3_remove_edge_had2_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), g.node_count());
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&g));
    }

    #[test]
    fn fixpoint_rewrites_every_site_and_terminates() {
        // Two disjoint had2 sites: {0,1,2} with normal leaves 3, 4, and
        // {5,6,7} with normal leaf 8. The fixpoint must consume both.
        let mut g = had2_site(0, 0, 0); // nodes 0..=4
        let v2 = g.add_vertex_with(VColor::Z(2));
        let a2 = g.add_vertex_with(VColor::Z(0));
        let b2 = g.add_vertex_with(VColor::Z(1));
        let r3 = g.add_vertex_with(VColor::Z(0));
        for (p, q) in [(v2, a2), (v2, b2), (a2, b2)] {
            g.add_edge_c(p, q, EColor::H);
        }
        g.add_edge_c(v2, r3, EColor::NC);

        let out = k3_remove_had2(&g);
        // 9 original vertices + u, w, x per site = 15.
        assert_eq!(out.node_count(), 15);
        // Both diagonals are gone and the v's have reset to Z(0).
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            0
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(6), NodeIndex::new(7)),
            0
        );
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(5)), VColor::Z(0));
        // w's carry the original v phases: 0 for site 1, 2 for site 2.
        assert_eq!(out.label(NodeIndex::new(10)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(13)), VColor::Z(2));
        // The two X(7) nodes close each chain.
        assert_eq!(out.label(NodeIndex::new(11)), VColor::X(7));
        assert_eq!(out.label(NodeIndex::new(14)), VColor::X(7));

        // Idempotent: a second fixpoint pass changes nothing.
        let twice = k3_remove_had2(&out);
        assert_eq!(out.node_count(), twice.node_count());
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&twice));
    }
}
