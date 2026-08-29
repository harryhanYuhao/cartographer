//! This file defines zx fuse operations
//! The fuse operation is defined below
//! if an Z(s) node `v_1` is connected to another Z(t) node `v_2`
//! via a normal edge they can be fused in to a single Z(s+t) node `v`
//! The new node `v` will be connected to all the neighbors of `v_1` and `v_2`
//! via their respective edges.
//!
//! The same operation can be applied to X(_) nodes as well
//!
//! Implementation details:
//!
//! - The fused spider lives at `v_1`, labelled `Z((s+t) mod 8)` or
//!   `X((s+t) mod 8)`; `v_2` is logically deleted. Phases add mode 8.
//! - Every edge at `v_2` (parallel copies included) is re-attached to `v_1`
//!   with its original colour. All edges between `v_1` and `v_2` (normal or
//!   H) and all self-loops at `v_2` are canceled by the fusion: the merge
//!   never leaves a self-loop on the fused spider.
//! - `fuse_total` normalizes H-edge parity (see [`normalize_h_parity_total`])
//!   before the first fusion and after every one.

use petgraph::graph::NodeIndex;

use crate::graph::{Graph, VColor};
use crate::operation::{
    utils::{get_normal_neighbour, nc_connected},
    zx::normalize_h_parity::normalize_h_parity_total,
};

/// A valid fuse pair: two distinct alive spiders of the same colour (both Z
/// or both X) joined by at least one normal edge.
fn valid_fuse_vertices(g: &Graph, v1: NodeIndex, v2: NodeIndex) -> bool {
    if v1 == v2 {
        return false;
    }
    if !g.is_alive(v1) || !g.is_alive(v2) {
        return false;
    }
    if !matches!(
        (g.label(v1), g.label(v2)),
        (VColor::Z(_), VColor::Z(_)) | (VColor::X(_), VColor::X(_))
    ) {
        return false;
    }
    nc_connected(g, v1, v2)
}

/// The first valid fuse pair, if any: ascending `v1`, then its ascending
/// normal neighbours.
fn has_valid_fuse(g: &Graph) -> Option<(NodeIndex, NodeIndex)> {
    for v1 in g.alive_vertices() {
        for v2 in get_normal_neighbour(g, v1) {
            if valid_fuse_vertices(g, v1, v2) {
                return Some((v1, v2));
            }
        }
    }
    None
}

/// Fuse the same-colour spiders at `v1` and `v2` (phases `s`, `t`) into the
/// single spider `Z((s+t) mod 8)` (or `X((s+t) mod 8)`) living at `v1`;
/// invalid pairs return the graph unchanged.
pub fn fuse_vertices(g: &Graph, v1: NodeIndex, v2: NodeIndex) -> Graph {
    if !valid_fuse_vertices(g, v1, v2) {
        return g.clone();
    }
    let merged = match (g.label(v1), g.label(v2)) {
        (VColor::Z(s), VColor::Z(t)) => VColor::Z((s + t) % 8),
        (VColor::X(s), VColor::X(t)) => VColor::X((s + t) % 8),
        // Unreachable: valid_fuse_vertices checked the colours.
        _ => return g.clone(),
    };

    let mut out = g.clone();
    for (s, t, e) in g.edges() {
        if s != v2 && t != v2 {
            continue; // edge not at v2: untouched
        }
        // The endpoint that is not v2; a v2 self-loop has both.
        let other = if s == v2 { t } else { s };
        // Pair edges (normal or H) and v2 self-loops are canceled; every
        // other edge moves onto the fused spider with its colour.
        if other != v2 && other != v1 {
            out.add_edge_c(v1, other, g.edge_color(e));
        }
    }
    // Cancel every edge of the pair (normal or H), then retire v2.
    while out.edge_multiplicity(v1, v2) > 0 {
        out.remove_edge(v1, v2);
    }
    out.remove_vertex(v2);
    out.set_color(v1, merged);
    out
}

/// Apply fuse_vertices repeatedly until no valid pair remains, normalizing
/// H-edge parity ([`normalize_h_parity_total`]) before the first fusion and
/// after every one.
pub fn fuse_total(g: &Graph) -> Graph {
    let mut tmp = g.clone();
    tmp = normalize_h_parity_total(&tmp);
    while let Some((v1, v2)) = has_valid_fuse(&tmp) {
        tmp = fuse_vertices(&tmp, v1, v2);
        tmp = normalize_h_parity_total(&tmp);
    }
    tmp
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::EColor;
    use crate::operation::zx::unfuse::unfuse_to_sep_h_edge;

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
    fn fuses_z_pair_and_wraps_phase() {
        // v1=Z(3) == v2=Z(5); v1 also touches a by H, v2 touches b by two
        // parallel normal edges and c by H.
        let mut g = Graph::new();
        let v1 = g.add_vertex_with(VColor::Z(3));
        let v2 = g.add_vertex_with(VColor::Z(5));
        let a = g.add_vertex_with(VColor::Z(0));
        let b = g.add_vertex_with(VColor::X(2));
        let c = g.add_vertex_with(VColor::Z(1));
        g.add_edge_c(v1, v2, EColor::NC);
        g.add_edge_c(v1, a, EColor::H);
        g.add_edge_c(v2, b, EColor::NC);
        g.add_edge_c(v2, b, EColor::NC);
        g.add_edge_c(v2, c, EColor::H);

        let out = fuse_vertices(&g, v1, v2);

        // v2 is logically deleted; the fused spider sits at v1 as Z(3+5=8=0).
        assert_eq!(out.node_count(), 5);
        assert_eq!(out.alive_count(), 4);
        assert!(!out.is_alive(v2));
        assert_eq!(out.label(v1), VColor::Z(0));

        // v1 keeps its own H edge and inherits both parallel copies of the
        // v2-b edge plus the v2-c H edge, each with its colour. The pair's
        // normal edge is consumed.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 2, EColor::H),
                (0, 3, EColor::NC),
                (0, 3, EColor::NC),
                (0, 4, EColor::H),
            ]
        );

        // Input graph is left untouched.
        assert_eq!(g.alive_count(), 5);
        assert_eq!(sorted_colored_edges(&g).len(), 5);
    }

    #[test]
    fn fuses_x_pair() {
        let mut g = Graph::new();
        let v1 = g.add_vertex_with(VColor::X(2));
        let v2 = g.add_vertex_with(VColor::X(7));
        let z = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v1, v2, EColor::NC);
        g.add_edge_c(v2, z, EColor::H);

        let out = fuse_vertices(&g, v1, v2);
        assert_eq!(out.label(v1), VColor::X((2 + 7) % 8));
        assert!(!out.is_alive(v2));
        assert_eq!(out.alive_count(), 2);
        assert_eq!(sorted_colored_edges(&out), vec![(0, 2, EColor::H)]);
    }

    #[test]
    fn h_pair_edges_are_canceled() {
        // Two H copies ride along the normal fuse edge: the fusion cancels
        // them instead of leaving H self-loops behind.
        let mut g = Graph::new();
        let v1 = g.add_vertex_with(VColor::Z(1));
        let v2 = g.add_vertex_with(VColor::Z(2));
        g.add_edge_c(v1, v2, EColor::H);
        g.add_edge_c(v1, v2, EColor::H);
        g.add_edge_c(v1, v2, EColor::NC);

        let out = fuse_vertices(&g, v1, v2);
        assert_eq!(out.label(v1), VColor::Z(3));
        assert_eq!(out.alive_count(), 1);
        assert_eq!(out.edge_multiplicity(v1, v1), 0);
        assert_eq!(sorted_colored_edges(&out), vec![]);
    }

    #[test]
    fn v2_self_loops_are_canceled() {
        // Self-loops at v2, H or normal alike, never reach the fused spider;
        // the ordinary v2-n edge still moves over with its colour.
        let mut g = Graph::new();
        let v1 = g.add_vertex_with(VColor::Z(0));
        let v2 = g.add_vertex_with(VColor::Z(4));
        let n = g.add_vertex_with(VColor::X(0));
        g.add_edge_c(v1, v2, EColor::NC);
        g.add_edge_c(v2, v2, EColor::H);
        g.add_edge_c(v2, v2, EColor::NC);
        g.add_edge_c(v2, n, EColor::H);

        let out = fuse_vertices(&g, v1, v2);
        assert_eq!(out.label(v1), VColor::Z(4));
        assert_eq!(out.edge_multiplicity(v1, v1), 0);
        assert_eq!(sorted_colored_edges(&out), vec![(0, 2, EColor::H)]);
    }

    #[test]
    fn consumes_all_parallel_nc_pair_edges() {
        let mut g = Graph::new();
        let v1 = g.add_vertex_with(VColor::Z(0));
        let v2 = g.add_vertex_with(VColor::Z(0));
        for _ in 0..3 {
            g.add_edge_c(v1, v2, EColor::NC);
        }

        let out = fuse_vertices(&g, v1, v2);
        assert_eq!(out.alive_count(), 1);
        assert_eq!(out.edge_multiplicity(v1, v2), 0);
        // No self-loops are left behind by the consumed copies.
        assert_eq!(sorted_colored_edges(&out), vec![]);
        assert_eq!(out.label(v1), VColor::Z(0));
    }

    #[test]
    fn returns_unchanged_for_invalid_pairs() {
        // 0=Z, 1=X, 2=Z, 3=H: mixed colours, an H-only pair, a non-spider,
        // and the self-pair are all invalid.
        let mut g = Graph::new();
        let z = g.add_vertex_with(VColor::Z(0));
        let x = g.add_vertex_with(VColor::X(0));
        let z2 = g.add_vertex_with(VColor::Z(0));
        let h = g.add_vertex_with(VColor::H);
        g.add_edge_c(z, x, EColor::NC);
        g.add_edge_c(z, z2, EColor::H);
        g.add_edge_c(z, h, EColor::NC);

        for (a, b) in [(z, x), (z, z2), (z, h), (z, z), (x, z2)] {
            let out = fuse_vertices(&g, a, b);
            assert_eq!(out.alive_count(), 4, "pair ({:?}, {:?})", a, b);
            assert_eq!(
                sorted_colored_edges(&out),
                sorted_colored_edges(&g),
                "pair ({:?}, {:?})",
                a,
                b
            );
        }

        // A logically deleted endpoint cannot fuse.
        let mut g2 = g.clone();
        g2.remove_vertex(z2);
        let out = fuse_vertices(&g2, z, z2);
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&g2));
    }

    #[test]
    fn fixpoint_fuses_nc_chains_and_is_idempotent() {
        // NC chain 0==1==2 of Z's, NC pair 3==4 of X's, plus a Z hung off 0
        // by an H edge (which must never fuse).
        let mut g = Graph::new();
        for p in [1u8, 2, 3] {
            g.add_vertex_with(VColor::Z(p));
        }
        g.add_vertex_with(VColor::X(1));
        g.add_vertex_with(VColor::X(1));
        let z = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::NC);
        g.add_edge_c(NodeIndex::new(1), NodeIndex::new(2), EColor::NC);
        g.add_edge_c(NodeIndex::new(3), NodeIndex::new(4), EColor::NC);
        g.add_edge_c(NodeIndex::new(0), z, EColor::H);

        let out = fuse_total(&g);
        // 1+2+3 = 6 at node 0; 1+1 = 2 at node 3; the H edge survives.
        assert_eq!(out.alive_count(), 3);
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(6));
        assert_eq!(out.label(NodeIndex::new(3)), VColor::X(2));
        assert_eq!(out.label(z), VColor::Z(0));
        assert_eq!(sorted_colored_edges(&out), vec![(0, 5, EColor::H)]);

        // Idempotent: a second fixpoint pass changes nothing.
        let twice = fuse_total(&out);
        assert_eq!(out.alive_count(), twice.alive_count());
        assert_eq!(sorted_colored_edges(&twice), sorted_colored_edges(&out));
    }

    #[test]
    fn fuse_total_inverts_unfuse() {
        // v=Z(3) with normal neighbours a1=X(5), a2=NC and H neighbour b:
        // unfusing and fusing everything back must reproduce the diagram.
        // (a1, a2 must not be Z spiders, or fuse_total would keep going.)
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(3));
        let a1 = g.add_vertex_with(VColor::X(5));
        let a2 = g.add_vertex_with(VColor::NC);
        let b = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v, a1, EColor::NC);
        g.add_edge_c(v, a2, EColor::NC);
        g.add_edge_c(v, b, EColor::H);

        let unfused = unfuse_to_sep_h_edge(&g, v);
        let back = fuse_total(&unfused);

        assert_eq!(back.alive_count(), 4);
        assert_eq!(back.label(v), VColor::Z(3));
        assert_eq!(sorted_colored_edges(&back), sorted_colored_edges(&g));
        assert_eq!(back.to_graph3(), g.to_graph3());
    }
}
