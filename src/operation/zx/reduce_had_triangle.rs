//! Reduce Hadamard triangle: delete the H edge of an all-Z H triangle.
//!
//! The rule targets a triangle `v1 -- v2 -- v3 -- v1` of Hadamard edges in
//! which all three vertices are Z spiders, and removes the edge `v2 -- v3` in
//! three steps:
//!
//! 1. Normalize H-edge parity on the whole graph
//!    ([`normalize_h_parity_total`]): parallel H edges cancel in pairs
//!    (`H * H = I`), so every H-connected pair collapses to a single
//!    equivalent copy (odd) or vanishes (even), and every self-loop is
//!    removed.
//! 2. Split `v1 = Z(s)` apart (a wide unfuse). A fresh `w = Z(s)` takes over
//!    every edge between `v1` and `N1` — the neighbours of `v1` other than
//!    `v2` and `v3`, normal or H, parallel copies included, each keeping its
//!    colour — all `v1`-`N1` edges are deleted, `v1` and `w` are joined by a
//!    normal edge, and `v1`'s phase resets to zero.
//! 3. Apply [`k3_remove_edge_had2_on_vertex`] to `v1`, which is now a valid
//!    had2 site: its H neighbourhood is exactly `{v2, v3}`. This deletes the
//!    diagonal for good and bumps the phases of `v2` and `v3` by one step
//!    (`s*pi/4`, mode 8) once.
//!
//! Net effect per application (four fresh vertices `w`, `u`, `w'`, `x`):
//!
//! ```text
//! N1 --(their colours)-- w(Z(s)) == u(Z(0)) == w'(Z(0)) == x(X(7)) == v1(Z(0))
//! ```
//!
//! with the H edges `v1 -- v2` and `v1 -- v3` kept and `v2 -- v3` gone.
//!
//! Implementation details:
//!
//! - The pair `(v2, v3)` is the first ascending pair of `v1`'s H neighbours
//!   that closes an all-Z H triangle, each side counted with odd H parity;
//!   through a vertex in several triangles (an H-K4, say) the first one is
//!   reduced and the fixpoint keeps going.
//! - The parity normalization runs on the whole graph, not just the triangle:
//!   even-H pairs vanish everywhere, so an even-H neighbour of `v1` is no
//!   longer adjacent when the split happens and contributes nothing to `N1`.
//! - Self-loops are not neighbours, and the normalization removes them
//!   outright, whatever their colour.
//! - A pair carrying both NC and H edges (a mixed pair) is outside the
//!   intended graph-like input; its NC copies survive the parity
//!   normalization, and the diagonal's last edge is dropped by
//!   `k3_remove_edge_had2_on_vertex`'s `remove_edge`, which picks an
//!   arbitrary copy of the pair.

use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};
use crate::operation::utils::get_h_neighbour;
use crate::operation::zx::fuse::fuse_total;
use crate::operation::zx::k3_remove_edge_ha_2::k3_remove_edge_had2_on_vertex;
use crate::operation::zx::normalize_h_parity::{normalize_h_parity_total, odd_h_connected};

/// The first ascending pair `(v2, v3)` of `v1`'s H neighbours that closes an
/// all-Z triangle whose three sides each carry an odd number of H edges.
/// Assumes nothing about `v1` itself; dead or non-Z endpoints yield `None`.
fn find_h_triangle(g: &Graph, v1: NodeIndex) -> Option<(NodeIndex, NodeIndex)> {
    let nbrs: Vec<NodeIndex> = get_h_neighbour(g, v1)
        .into_iter()
        .filter(|&n| matches!(g.label(n), VColor::Z(_)) && odd_h_connected(g, v1, n))
        .collect();
    for (i, &v2) in nbrs.iter().enumerate() {
        for &v3 in &nbrs[i + 1..] {
            if odd_h_connected(g, v2, v3) {
                return Some((v2, v3));
            }
        }
    }
    None
}

/// A valid reduce target: an alive Z spider sitting in an all-Z H triangle.
fn valid_reduce_vertex(g: &Graph, v1: NodeIndex) -> bool {
    g.is_alive(v1) && matches!(g.label(v1), VColor::Z(_)) && find_h_triangle(g, v1).is_some()
}

fn has_valid_reduce_vertex(g: &Graph) -> Option<NodeIndex> {
    g.alive_vertices().find(|&v| valid_reduce_vertex(g, v))
}

/// Reduce the Hadamard triangle at `v1` (see the module docs): normalize
/// H-edge parity on the whole graph, split `v1`'s neighbourhood off onto a
/// fresh `Z(s)` spider, then apply [`k3_remove_edge_had2_on_vertex`] to
/// remove the diagonal `v2 -- v3`. An invalid `v1` returns the graph
/// unchanged (without normalizing).
pub fn reduce_had_triangle_on_vertex(g: &Graph, v1: NodeIndex) -> Graph {
    let (s, v2, v3) = match (g.label(v1), find_h_triangle(g, v1)) {
        (VColor::Z(s), Some((v2, v3))) if g.is_alive(v1) => (s, v2, v3),
        _ => return g.clone(),
    };

    // Normalize H-edge parity everywhere first: every H-connected pair
    // collapses to a single copy (odd) or vanishes (even).
    let norm = normalize_h_parity_total(g);

    // N1: every neighbour of v1 other than v2, v3 (any edge colour), in the
    // normalized graph — even-H neighbours are gone by now.
    let n1: Vec<NodeIndex> = norm
        .alive_neighbors(v1)
        .into_iter()
        .filter(|&n| n != v2 && n != v3)
        .collect();

    // w takes over the N1 edges: every parallel copy, with its colour.
    let mut out = norm;
    let moved: Vec<(NodeIndex, EColor)> = out
        .edges()
        .filter(|&(a, b, _)| (a == v1) != (b == v1))
        .map(|(a, b, e)| (if a == v1 { b } else { a }, out.edge_color(e)))
        .filter(|&(other, _)| other != v2 && other != v3)
        .collect();
    let w = out.add_vertex_with(VColor::Z(s));
    for (other, c) in moved {
        out.add_edge_c(w, other, c);
    }
    // Delete every v1-N1 edge, bridge v1 to w by a normal edge, and move the
    // phase onto w.
    for &n in &n1 {
        while out.edge_multiplicity(v1, n) > 0 {
            out.remove_edge(v1, n);
        }
    }
    out.add_edge_c(v1, w, EColor::NC);
    out.set_color(v1, VColor::Z(0));

    // v1 is now a valid had2 site: its H neighbourhood is exactly {v2, v3}
    // and the normalized diagonal is a single H edge.
    k3_remove_edge_had2_on_vertex(&out, v1)
}

/// Apply reduce_had_triangle_on_vertex repeatedly until no all-Z H triangle
/// remains, running [`fuse_total`] (H-parity normalization + NC fusion)
/// before the first application and after every one; each fresh
/// `w == u == w'` chain collapses back into a single Z(s) spider. Every
/// application deletes an entire (odd) diagonal, fusions only remove
/// vertices, and no H edges are ever added, so the loop terminates.
pub fn reduce_had_triangle_total(g: &Graph) -> Graph {
    let mut tmp = g.clone();
    tmp = fuse_total(&tmp);
    while let Some(v) = has_valid_reduce_vertex(&tmp) {
        tmp = reduce_had_triangle_on_vertex(&tmp, v);
        tmp = fuse_total(&tmp);
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

    /// (H, NC) self-loop counts at `v`.
    fn self_loop_counts(g: &Graph, v: NodeIndex) -> (usize, usize) {
        let mut counts = (0, 0);
        for (s, t, e) in g.edges() {
            if s == v && t == v {
                if g.edge_color(e) == EColor::H {
                    counts.0 += 1;
                } else {
                    counts.1 += 1;
                }
            }
        }
        counts
    }

    /// v1=Z(pv) in an H triangle with v2=Z(0), v3=Z(0).
    fn h_triangle(pv: u8) -> Graph {
        let mut g = Graph::new();
        for p in [pv, 0, 0] {
            g.add_vertex_with(VColor::Z(p));
        }
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            g.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::H);
        }
        g
    }

    #[test]
    fn rewrites_triangle_moving_mixed_boundary() {
        // v1=Z(3), triangle v2=Z(1), v3=Z(5), normal leaf r=X(2), and an H
        // leaf h=Z(0) outside the triangle.
        let mut g = h_triangle(3);
        g.set_color(NodeIndex::new(1), VColor::Z(1));
        g.set_color(NodeIndex::new(2), VColor::Z(5));
        let r = g.add_vertex_with(VColor::X(2));
        let h = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(NodeIndex::new(0), r, EColor::NC);
        g.add_edge_c(NodeIndex::new(0), h, EColor::H);

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));

        // Prep appends w = 5; had2's unfuse appends u = 6, w' = 7; then x = 8.
        assert_eq!(out.node_count(), 9);
        // Chain r,h -- w(Z3) == u(Z0) == w'(Z0) == x(X7) == v1(Z0); the two H
        // edges at v1 are kept, the diagonal is gone, h keeps its H colour.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 8, EColor::NC),
                (3, 5, EColor::NC),
                (4, 5, EColor::H),
                (5, 6, EColor::NC),
                (6, 7, EColor::NC),
                (7, 8, EColor::NC),
            ]
        );
        // Labels: v1 resets to Z(0), w carries v1's old phase 3, the triangle
        // neighbours bump by one, everyone else is untouched.
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(2)); // 1 + 1
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(6)); // 5 + 1
        assert_eq!(out.label(NodeIndex::new(3)), VColor::X(2));
        assert_eq!(out.label(NodeIndex::new(4)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(5)), VColor::Z(3)); // w
        assert_eq!(out.label(NodeIndex::new(6)), VColor::Z(0)); // u
        assert_eq!(out.label(NodeIndex::new(7)), VColor::Z(0)); // w'
        assert_eq!(out.label(NodeIndex::new(8)), VColor::X(7)); // x

        // Input graph is left untouched.
        assert_eq!(g.node_count(), 5);
        assert_eq!(
            sorted_colored_edges(&g),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 3, EColor::NC),
                (0, 4, EColor::H),
                (1, 2, EColor::H),
            ]
        );
    }

    #[test]
    fn bare_triangle_appends_the_chain_only() {
        // Empty N1: w still splits off, and the chain w == u == w' == x == v1
        // is all the rewrite adds.
        let mut g = h_triangle(4);
        g.set_color(NodeIndex::new(2), VColor::Z(7));

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 7);
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 6, EColor::NC),
                (3, 4, EColor::NC),
                (4, 5, EColor::NC),
                (5, 6, EColor::NC),
            ]
        );
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(0)); // 7 + 1 wraps
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(4)); // w keeps s
        assert_eq!(out.label(NodeIndex::new(6)), VColor::X(7)); // x
    }

    #[test]
    fn parallel_h_edges_reduce_by_parity() {
        // Sides v1-v2 and the diagonal carry 3 H copies each (odd == one);
        // the normalization collapses them, the rewrite deletes the diagonal
        // for good, with a single phase bump.
        let mut g = h_triangle(0);
        g.set_color(NodeIndex::new(1), VColor::Z(2));
        for _ in 0..2 {
            g.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);
            g.add_edge_c(NodeIndex::new(1), NodeIndex::new(2), EColor::H);
        }

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 7);
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            1
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)),
            1
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            0
        );
        // Bumped once, not three times.
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(3));
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(1));
    }

    #[test]
    fn even_h_edges_do_not_form_a_triangle() {
        // Two H copies on the diagonal (even == no edge): no triangle, at any
        // corner, so no rewrite fires. The fixpoint still normalizes H parity
        // (its fuse_total pass starts with one): the even pair itself
        // vanishes from the total's output.
        let mut even_diagonal = h_triangle(0);
        even_diagonal.add_edge_c(NodeIndex::new(1), NodeIndex::new(2), EColor::H);
        let mut even_side = h_triangle(0);
        even_side.add_edge_c(NodeIndex::new(0), NodeIndex::new(1), EColor::H);

        for g in [&even_diagonal, &even_side] {
            for target in 0..3usize {
                let out = reduce_had_triangle_on_vertex(g, NodeIndex::new(target));
                assert_eq!(out.node_count(), g.node_count(), "target {target}");
                assert_eq!(
                    sorted_colored_edges(&out),
                    sorted_colored_edges(g),
                    "target {target}"
                );
            }
        }
        assert_eq!(
            sorted_colored_edges(&reduce_had_triangle_total(&even_diagonal)),
            vec![(0, 1, EColor::H), (0, 2, EColor::H)]
        );
        assert_eq!(
            sorted_colored_edges(&reduce_had_triangle_total(&even_side)),
            vec![(0, 2, EColor::H), (1, 2, EColor::H)]
        );
    }

    #[test]
    fn returns_unchanged_for_invalid_targets() {
        // (a) v1 not a Z spider; (b) v2 not a Z spider; (c) NC-only triangle;
        // (d) no diagonal (open path); (e) dead v1.
        let mut non_z_v1 = h_triangle(0);
        non_z_v1.set_color(NodeIndex::new(0), VColor::X(0));
        let mut non_z_v2 = h_triangle(0);
        non_z_v2.set_color(NodeIndex::new(1), VColor::X(3));

        let mut nc_triangle = Graph::new();
        for _ in 0..3 {
            nc_triangle.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(0usize, 1usize), (0, 2), (1, 2)] {
            nc_triangle.add_edge_c(NodeIndex::new(p), NodeIndex::new(q), EColor::NC);
        }

        let mut open = h_triangle(0);
        open.remove_edge(NodeIndex::new(1), NodeIndex::new(2));

        let mut dead_v1 = h_triangle(0);
        dead_v1.remove_vertex(NodeIndex::new(0));

        for g in [&non_z_v1, &non_z_v2, &nc_triangle, &open, &dead_v1] {
            let out = reduce_had_triangle_on_vertex(g, NodeIndex::new(0));
            assert_eq!(out.node_count(), g.node_count());
            assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(g));
        }
    }

    #[test]
    fn removes_v1_self_loops() {
        // The whole-graph parity pass deletes every self-loop (colour and
        // copies alike) before the split; the rewrite itself proceeds.
        let mut g = h_triangle(1);
        let r = g.add_vertex_with(VColor::X(0));
        g.add_edge_c(NodeIndex::new(0), r, EColor::NC);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::H);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::H);
        g.add_edge_c(NodeIndex::new(0), NodeIndex::new(0), EColor::NC);

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 8);
        assert_eq!(self_loop_counts(&out, NodeIndex::new(0)), (0, 0));
        // The rest of the rewrite still happened: diagonal gone, chain built.
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            0
        );
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(4)), VColor::Z(1)); // w keeps s=1
        assert_eq!(out.label(NodeIndex::new(7)), VColor::X(7)); // x
    }

    #[test]
    fn normalizes_h_parity_before_moving_n1() {
        // The whole-graph parity pass runs before the split: the even-H pair
        // (v1, h) vanishes so h never reaches w, the odd-H pair (v1, q)
        // collapses to one copy and moves, NC copies move untouched.
        let mut g = h_triangle(2);
        let r = g.add_vertex_with(VColor::X(1)); // 3: two NC copies
        let h = g.add_vertex_with(VColor::Z(0)); // 4: two H copies (even)
        let q = g.add_vertex_with(VColor::X(0)); // 5: three H copies (odd)
        g.add_edge_c(NodeIndex::new(0), r, EColor::NC);
        g.add_edge_c(NodeIndex::new(0), r, EColor::NC);
        g.add_edge_c(NodeIndex::new(0), h, EColor::H);
        g.add_edge_c(NodeIndex::new(0), h, EColor::H);
        g.add_edge_c(NodeIndex::new(0), q, EColor::H);
        g.add_edge_c(NodeIndex::new(0), q, EColor::H);
        g.add_edge_c(NodeIndex::new(0), q, EColor::H);

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 10);
        // w = 6 inherits both NC copies to r and the single surviving H copy
        // to q; h is detached from everything.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 9, EColor::NC),
                (3, 6, EColor::NC),
                (3, 6, EColor::NC),
                (5, 6, EColor::H),
                (6, 7, EColor::NC),
                (7, 8, EColor::NC),
                (8, 9, EColor::NC),
            ]
        );
        assert_eq!(out.edge_multiplicity(NodeIndex::new(0), h), 0);
        assert_eq!(out.edge_multiplicity(NodeIndex::new(6), h), 0);
        assert_eq!(out.label(NodeIndex::new(6)), VColor::Z(2)); // w keeps s
        assert_eq!(out.label(h), VColor::Z(0)); // h itself survives, isolated
        assert_eq!(out.label(NodeIndex::new(9)), VColor::X(7)); // x
    }

    #[test]
    fn chooses_first_ascending_pair() {
        // H-K4 on 0..=3: reducing at 0 picks the pair (1, 2); vertex 3 joins
        // N1 and its H edge moves onto w undiminished.
        let mut g = h_triangle(0);
        let d = g.add_vertex_with(VColor::Z(0));
        for p in [0usize, 1, 2] {
            g.add_edge_c(NodeIndex::new(p), d, EColor::H);
        }

        let out = reduce_had_triangle_on_vertex(&g, NodeIndex::new(0));
        assert_eq!(out.node_count(), 8);
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 7, EColor::NC),
                (1, 3, EColor::H),
                (2, 3, EColor::H),
                (3, 4, EColor::H),
                (4, 5, EColor::NC),
                (5, 6, EColor::NC),
                (6, 7, EColor::NC),
            ]
        );
        // Only the reduced triangle's corners bump; the N1 member 3 does not.
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(0));
    }

    #[test]
    fn fixpoint_rewrites_fuses_and_is_idempotent() {
        // Two disjoint triangles: the fixpoint consumes both diagonals and,
        // since it interleaves fuse_total, collapses each fresh w == u == w'
        // chain back into the single w carrying v's phase.
        let mut g = h_triangle(1); // nodes 0..=2, v = Z(1)
        let v2 = g.add_vertex_with(VColor::Z(2));
        let a2 = g.add_vertex_with(VColor::Z(0));
        let b2 = g.add_vertex_with(VColor::Z(1));
        for (p, q) in [(v2, a2), (v2, b2), (a2, b2)] {
            g.add_edge_c(p, q, EColor::H);
        }

        let out = reduce_had_triangle_total(&g);
        // 6 original vertices + 4 per site; the fused-away chain links
        // (7, 8, 11, 12) are logically dead.
        assert_eq!(out.node_count(), 14);
        assert_eq!(out.alive_count(), 10);
        for dead in [7usize, 8, 11, 12] {
            assert!(!out.is_alive(NodeIndex::new(dead)));
        }
        // Both diagonals are gone; each chain collapsed to w == x.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 1, EColor::H),
                (0, 2, EColor::H),
                (0, 9, EColor::NC),
                (3, 4, EColor::H),
                (3, 5, EColor::H),
                (3, 13, EColor::NC),
                (6, 9, EColor::NC),
                (10, 13, EColor::NC),
            ]
        );
        // Phases bumped once per site, v's reset to Z(0), the fused w's
        // carry the original v phases (1 and 2), the x's are X(7).
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            0
        );
        assert_eq!(
            out.edge_multiplicity(NodeIndex::new(4), NodeIndex::new(5)),
            0
        );
        assert_eq!(out.label(NodeIndex::new(0)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(1)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(4)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(5)), VColor::Z(2));
        assert_eq!(out.label(NodeIndex::new(6)), VColor::Z(1));
        assert_eq!(out.label(NodeIndex::new(10)), VColor::Z(2));
        assert_eq!(out.label(NodeIndex::new(9)), VColor::X(7));
        assert_eq!(out.label(NodeIndex::new(13)), VColor::X(7));

        // Idempotent: no all-Z H triangle remains and nothing left to fuse.
        let twice = reduce_had_triangle_total(&out);
        assert_eq!(out.node_count(), twice.node_count());
        assert_eq!(out.alive_count(), twice.alive_count());
        assert_eq!(sorted_colored_edges(&out), sorted_colored_edges(&twice));
    }
}
