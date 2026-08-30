//! The line graph: edges become vertices, shared endpoints become edges.
//!
//! The line graph `G*` of `G` has one vertex per edge of `G` — isolated
//! vertices of `G` (having no edges) do not appear. Two vertices of `G*`
//! are joined by an edge iff the corresponding edges of `G` share an
//! endpoint.
//!
//! This crate's colours are ignored: every vertex of `G*` is `NC` and every
//! edge `NC` (the plain graph-theoretic line graph), and the adjacency is
//! boolean — two parallel edges of `G`, sharing both endpoints, are still
//! joined by a single edge in `G*`. A self-loop `{v, v}` of `G` is adjacent
//! to every other edge at `v` (it shares `v` with them) but never yields a
//! self-loop in `G*`.

use petgraph::graph::NodeIndex;
use std::collections::HashMap;

use crate::graph::{EColor, Graph};

/// Build the line graph `G*` of `g` (see the module docs). Vertices of `G*`
/// are numbered in `g`'s alive-edge order.
pub fn line_graph(g: &Graph) -> Graph {
    // Snapshot the alive edges: vertex i of G* is edges[i] of g.
    let edges: Vec<(NodeIndex, NodeIndex)> = g.edges().map(|(s, t, _)| (s, t)).collect();

    let mut out = Graph::new();
    let nodes: Vec<NodeIndex> = edges.iter().map(|_| out.add_vertex()).collect();

    // Group G* vertices by the endpoint of g they share; a self-loop is
    // registered once at its vertex.
    let mut at: HashMap<NodeIndex, Vec<usize>> = HashMap::new();
    for (i, &(s, t)) in edges.iter().enumerate() {
        at.entry(s).or_default().push(i);
        if t != s {
            at.entry(t).or_default().push(i);
        }
    }

    // Walk the endpoints in ascending order so the output is deterministic.
    let mut groups: Vec<(NodeIndex, Vec<usize>)> = at.into_iter().collect();
    groups.sort_unstable_by_key(|&(v, _)| v);
    for (_v, group) in groups {
        // Two G* vertices are joined iff they meet at an endpoint; skip
        // pairs already joined (parallel edges share two endpoints).
        for i in 0..group.len() {
            for j in (i + 1)..group.len() {
                let (a, b) = (nodes[group[i]], nodes[group[j]]);
                if !out.has_edge(a, b) {
                    out.add_edge_c(a, b, EColor::NC);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::VColor;

    fn node(i: usize) -> NodeIndex {
        NodeIndex::new(i)
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
    fn triangle_maps_to_triangle() {
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        for (p, q) in [(0, 1), (1, 2), (0, 2)] {
            g.add_edge_c(node(p), node(q), EColor::H);
        }

        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 3);
        // All three pairs of edges share a corner: G* is again a triangle.
        assert_eq!(
            sorted_colored_edges(&lg),
            vec![(0, 1, EColor::NC), (0, 2, EColor::NC), (1, 2, EColor::NC)]
        );
        // Plain line graph: no colours anywhere.
        for i in 0..3 {
            assert_eq!(lg.label(node(i)), VColor::NC);
        }

        // Input graph is left untouched.
        assert_eq!(
            sorted_colored_edges(&g),
            vec![(0, 1, EColor::H), (0, 2, EColor::H), (1, 2, EColor::H)]
        );
    }

    #[test]
    fn path_and_star() {
        // Path 0-1-2: two edges sharing vertex 1 -> one G* edge.
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        g.add_edge_c(node(0), node(1), EColor::NC);
        g.add_edge_c(node(1), node(2), EColor::NC);
        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 2);
        assert_eq!(sorted_colored_edges(&lg), vec![(0, 1, EColor::NC)]);

        // Star K1,3: all edges share the center -> G* is a triangle.
        let mut g = Graph::new();
        for _ in 0..4 {
            g.add_vertex_with(VColor::Z(0));
        }
        for leaf in 1..4 {
            g.add_edge_c(node(0), node(leaf), EColor::NC);
        }
        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 3);
        assert_eq!(
            sorted_colored_edges(&lg),
            vec![(0, 1, EColor::NC), (0, 2, EColor::NC), (1, 2, EColor::NC)]
        );
    }

    #[test]
    fn parallel_pair_gets_a_single_edge() {
        // Two parallel 0-1 edges share BOTH endpoints: boolean adjacency.
        let mut g = Graph::new();
        g.add_vertex_with(VColor::Z(0));
        g.add_vertex_with(VColor::Z(0));
        g.add_edge_c(node(0), node(1), EColor::NC);
        g.add_edge_c(node(0), node(1), EColor::H);

        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 2);
        assert_eq!(sorted_colored_edges(&lg), vec![(0, 1, EColor::NC)]);
    }

    #[test]
    fn loop_is_adjacent_to_other_edges_at_its_vertex() {
        // Loop {0,0}, edge {0,1} (shares 0 with the loop), lone loop {2,2}.
        let mut g = Graph::new();
        for _ in 0..3 {
            g.add_vertex_with(VColor::Z(0));
        }
        g.add_edge_c(node(0), node(0), EColor::NC);
        g.add_edge_c(node(0), node(1), EColor::NC);
        g.add_edge_c(node(2), node(2), EColor::NC);

        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 3);
        // The loop is adjacent to the edge at 0; the loop at 2 is isolated;
        // nothing creates a self-loop in G*.
        assert_eq!(sorted_colored_edges(&lg), vec![(0, 1, EColor::NC)]);
    }

    #[test]
    fn colours_are_ignored() {
        let mut h_version = Graph::new();
        let mut nc_version = Graph::new();
        for g in [&mut h_version, &mut nc_version] {
            for _ in 0..4 {
                g.add_vertex_with(VColor::Z(3));
            }
        }
        for (p, q) in [(0, 1), (1, 2), (2, 3), (1, 3)] {
            h_version.add_edge_c(node(p), node(q), EColor::H);
            nc_version.add_edge_c(node(p), node(q), EColor::NC);
        }
        assert_eq!(
            line_graph(&h_version).to_graph3(),
            line_graph(&nc_version).to_graph3()
        );
    }

    #[test]
    fn dead_and_isolated_vertices_contribute_nothing() {
        // Vertex colours/deadness only matter through alive edges: an edge
        // touching a dead vertex vanishes, and isolated vertices never had
        // edges to begin with.
        let mut g = Graph::new();
        for _ in 0..4 {
            g.add_vertex_with(VColor::Z(0));
        }
        g.add_edge_c(node(0), node(1), EColor::NC); // 0 will die
        g.add_edge_c(node(1), node(2), EColor::NC); // alive
        g.remove_vertex(node(0)); // vertex 3 is alive but isolated

        let lg = line_graph(&g);
        assert_eq!(lg.node_count(), 1);
        assert_eq!(sorted_colored_edges(&lg), Vec::new());
    }
}
