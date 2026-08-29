use crate::graph::{EColor, Graph};
use petgraph::graph::NodeIndex;

/// Is there a normal-coloured edge between `a` and `b`?
pub fn nc_connected(g: &Graph, a: NodeIndex, b: NodeIndex) -> bool {
    g.edges_at(a)
        .any(|(_, o, e)| o == b && g.edge_color(e) == EColor::NC)
}

/// Is there an H-coloured edge between `a` and `b`?
pub fn h_connected(g: &Graph, a: NodeIndex, b: NodeIndex) -> bool {
    g.edges_at(a)
        .any(|(_, o, e)| o == b && g.edge_color(e) == EColor::H)
}

pub fn get_h_neighbour(g: &Graph, v: NodeIndex) -> Vec<NodeIndex> {
    let mut n1: Vec<NodeIndex> = g
        .edges_at(v)
        .filter(|&(_, o, e)| o != v && g.edge_color(e) == EColor::H)
        .map(|(_, o, _)| o)
        .collect();

    n1.sort_unstable();
    n1.dedup();
    n1
}

pub fn get_normal_neighbour(g: &Graph, v: NodeIndex) -> Vec<NodeIndex> {
    let mut n1: Vec<NodeIndex> = g
        .edges_at(v)
        .filter(|&(_, o, e)| o != v && g.edge_color(e) == EColor::NC)
        .map(|(_, o, _)| o)
        .collect();

    n1.sort_unstable();
    n1.dedup();
    n1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(i: usize) -> NodeIndex {
        NodeIndex::new(i)
    }

    /// 0 joined to 1 (NC), 2 (H), 3 (2x parallel H); NC self-loop at 0, H
    /// self-loop at 2; 4 joined to 0 but logically deleted.
    fn fixture() -> Graph {
        let mut g = Graph::with_capacity(5);
        for _ in 0..5 {
            g.add_vertex();
        }
        g.add_edge_c(node(0), node(1), EColor::NC);
        g.add_edge_c(node(0), node(2), EColor::H);
        g.add_edge_c(node(0), node(3), EColor::H);
        g.add_edge_c(node(0), node(3), EColor::H);
        g.add_edge_c(node(0), node(0), EColor::NC);
        g.add_edge_c(node(2), node(2), EColor::H);
        g.add_edge_c(node(0), node(4), EColor::H);
        g.remove_vertex(node(4));
        g
    }

    #[test]
    fn connected_helpers_respect_colour_and_liveness() {
        let g = fixture();
        assert!(nc_connected(&g, node(0), node(1)));
        assert!(!h_connected(&g, node(0), node(1)));
        assert!(h_connected(&g, node(0), node(2)));
        assert!(!nc_connected(&g, node(0), node(2)));
        // Parallel copies: presence, not multiplicity.
        assert!(h_connected(&g, node(0), node(3)));
        // Self-loops count as a connection (colour included).
        assert!(nc_connected(&g, node(0), node(0)));
        assert!(h_connected(&g, node(2), node(2)));
        assert!(!h_connected(&g, node(0), node(0)));
        // Dead endpoints see nothing.
        assert!(!h_connected(&g, node(0), node(4)));
        assert!(!nc_connected(&g, node(0), node(4)));
        // Disconnected pair.
        assert!(!nc_connected(&g, node(1), node(2)));
        // Symmetric.
        assert!(h_connected(&g, node(2), node(0)));
    }

    #[test]
    fn neighbour_helpers_are_sorted_deduped_and_skip_loops() {
        let mut g = fixture();
        // More parallel NC copies and an H copy to 1, so 0 reaches 1 both ways.
        g.add_edge_c(node(0), node(1), EColor::NC);
        g.add_edge_c(node(0), node(1), EColor::H);

        assert_eq!(
            get_h_neighbour(&g, node(0)),
            vec![node(1), node(2), node(3)]
        );
        assert_eq!(get_normal_neighbour(&g, node(0)), vec![node(1)]);
        // Self-loops never appear; the H loop at 2 gives no H neighbour.
        assert_eq!(get_h_neighbour(&g, node(2)), vec![node(0)]);
        // Dead vertex: no edges visible at all.
        assert_eq!(get_h_neighbour(&g, node(4)), Vec::new());
        assert_eq!(get_normal_neighbour(&g, node(4)), Vec::new());
    }
}
