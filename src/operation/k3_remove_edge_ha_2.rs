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
    unfuse::unfuse_to_sep_h_edge,
    utils::{get_h_neighbour, h_connected},
};
use petgraph::graph::NodeIndex;

use crate::graph::{EColor, Graph, VColor};

fn valid_k3_vertex_degree2(g: &Graph, v: NodeIndex) -> bool {
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
    g.alive_vertices().find(|&v| valid_k3_vertex_degree2(g, v))
}

/// Remove an H edge from the first applicable K3 of Z spiders, if any.
/// TODO: UNFINISH
pub fn k3_remove_edge_had2_on_vertex(g: &Graph, v: NodeIndex) -> Graph {
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
pub fn k3_remove_edge_degree_2(g: &Graph) -> Graph {
    let mut tmp = g.clone();
    loop {
        match has_valid_k3_vertex_had2(&tmp) {
            Some(v) => tmp = k3_remove_edge_had2_on_vertex(g, v),
            None => break,
        }
    }
    tmp
}
