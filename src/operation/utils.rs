use crate::graph::{EColor, Graph}; 
use petgraph::graph::NodeIndex;

/// Is there an H-coloured edge between `a` and `b`?
pub fn h_connected(g: &Graph, a: NodeIndex, b: NodeIndex) -> bool {
    g.edges()
        .any(|(s, t, e)| g.edge_color(e) == EColor::H && ((s == a && t == b) || (s == b && t == a)))
}

pub fn get_h_neighbour(g: &Graph, v: NodeIndex) -> Vec<NodeIndex> {
    let mut n1: Vec<NodeIndex> = g
        .edges()
        .filter(|&(s, t, e)| g.edge_color(e) == EColor::NC && (s == v) != (t == v))
        .map(|(s, t, _)| if s == v { t } else { s })
        .collect();

    n1.sort_unstable();
    n1.dedup();
    n1
}
