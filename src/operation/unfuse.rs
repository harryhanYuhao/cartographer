//! Spider unfusing for a Z spider with a mixed (normal + Hadamard) boundary.

use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};

/// Split `v = Z(p)` so that its normal-edge neighbourhood hangs behind two
/// fresh spiders.
///
/// Precondition: `v` is an alive Z spider with phase `p`. Its alive
/// neighbours partition into `N1` (joined by a normal edge) and `N2` (joined
/// by an H edge). The rewrite turns
///
/// ```text
/// N1 == v(Z(p)) -- N2
/// ```
///
/// into
///
/// ```text
/// N1 == u(Z(0)) == w(Z(p)) == v(Z(0)) -- N2
/// ```
///
/// (`==` normal edge, `--` H edge): a new `u = Z(0)` takes over all of `N1`
/// by normal edges, a new `w = Z(p)` bridges `u` and `v` by normal edges,
/// every edge between `v` and `N1` is deleted, and `v`'s phase resets to
/// `Z(0)` (the full phase has moved onto `w`). Everything else, including
/// the `v`–`N2` H edges, is left unchanged.
pub fn unfuse_to_sep_h_edge(g: &Graph, v: NodeIndex) -> Graph {
    let p = match g.label(v) {
        VColor::Z(p) if g.is_alive(v) => p,
        _ => return g.clone(),
    };

    // N1: alive neighbours of v reached through a normal edge. Sorted +
    // deduped: edges() yields each parallel copy separately.
    let mut n1: Vec<NodeIndex> = g
        .edges()
        .filter(|&(s, t, e)| g.edge_color(e) == EColor::NC && (s == v) != (t == v))
        .map(|(s, t, _)| if s == v { t } else { s })
        .collect();
    n1.sort_unstable();
    n1.dedup();

    let mut out = g.clone();
    // u takes over the normal-edge neighbourhood; w carries the old phase.
    let u = out.add_vertex_with(VColor::Z(0));
    let w = out.add_vertex_with(VColor::Z(p));
    for &n in &n1 {
        out.add_edge_c(u, n, EColor::NC);
    }
    out.add_edge_c(w, u, EColor::NC);
    out.add_edge_c(w, v, EColor::NC);

    // Drop every edge between v and N1; v itself loses its phase to w.
    for &n in &n1 {
        while out.edge_multiplicity(v, n) > 0 {
            out.remove_edge(v, n);
        }
    }
    out.set_color(v, VColor::Z(0));

    out
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

    #[test]
    fn moves_normal_neighbours_behind_u_w_chain() {
        // v=Z(3) with normal neighbours a1, a2 and H neighbour b.
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(3));
        let a1 = g.add_vertex_with(VColor::Z(1));
        let a2 = g.add_vertex_with(VColor::X(2));
        let b = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v, a1, EColor::NC);
        g.add_edge_c(v, a2, EColor::NC);
        g.add_edge_c(v, b, EColor::H);

        let out = unfuse_to_sep_h_edge(&g, v);

        // Two new vertices: u = node 4, w = node 5.
        assert_eq!(out.node_count(), 6);
        assert_eq!(out.label(NodeIndex::new(4)), VColor::Z(0)); // u
        assert_eq!(out.label(NodeIndex::new(5)), VColor::Z(3)); // w keeps p

        // The v-N1 normal edges moved onto u; w chains u to v; the v-N2 H
        // edge is untouched.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 3, EColor::H),
                (0, 5, EColor::NC),
                (1, 4, EColor::NC),
                (2, 4, EColor::NC),
                (4, 5, EColor::NC),
            ]
        );
        // Existing labels are preserved, but v's phase moved onto w.
        assert_eq!(out.label(v), VColor::Z(0));
        assert_eq!(out.label(a1), VColor::Z(1));
        assert_eq!(out.label(a2), VColor::X(2));

        // Input is left untouched.
        assert_eq!(g.node_count(), 4);
        assert_eq!(
            sorted_colored_edges(&g),
            vec![(0, 1, EColor::NC), (0, 2, EColor::NC), (0, 3, EColor::H)]
        );
    }

    #[test]
    fn deletes_all_parallel_edges_to_n1() {
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(0));
        let a = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v, a, EColor::NC);
        g.add_edge_c(v, a, EColor::NC);

        let out = unfuse_to_sep_h_edge(&g, v);
        // Nothing survives between v and a; only the u-a, u-w, w-v chain.
        assert_eq!(out.edge_multiplicity(v, a), 0);
        assert_eq!(
            sorted_colored_edges(&out),
            vec![(0, 3, EColor::NC), (1, 2, EColor::NC), (2, 3, EColor::NC),]
        );
    }

    #[test]
    fn empty_n1_still_appends_the_chain() {
        // Pure H boundary: N1 is empty, yet u--w--v is still attached.
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(7));
        let b = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v, b, EColor::H);

        let out = unfuse_to_sep_h_edge(&g, v);
        assert_eq!(out.node_count(), 4);
        assert_eq!(
            sorted_colored_edges(&out),
            vec![(0, 1, EColor::H), (0, 3, EColor::NC), (2, 3, EColor::NC),]
        );
        assert_eq!(out.label(NodeIndex::new(2)), VColor::Z(0));
        assert_eq!(out.label(NodeIndex::new(3)), VColor::Z(7));
    }

    #[test]
    fn pure_normal_boundary_leaves_v_via_w_only() {
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::Z(1));
        let l1 = g.add_vertex_with(VColor::Z(0));
        let l2 = g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(v, l1, EColor::NC);
        g.add_edge_c(v, l2, EColor::NC);

        let out = unfuse_to_sep_h_edge(&g, v);
        // v now touches only w; the leaves hang off u.
        assert_eq!(
            sorted_colored_edges(&out),
            vec![
                (0, 4, EColor::NC),
                (1, 3, EColor::NC),
                (2, 3, EColor::NC),
                (3, 4, EColor::NC),
            ]
        );
        assert_eq!(out.degree(v), 1);
    }

    #[test]
    #[should_panic(expected = "v must be an alive Z spider")]
    fn rejects_non_z_target() {
        let mut g = Graph::new();
        let v = g.add_vertex_with(VColor::X(0));
        unfuse_to_sep_h_edge(&g, v);
    }
}
