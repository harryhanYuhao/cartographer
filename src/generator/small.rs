//! Common named graphs on at most ten vertices.
//!
//! These are fixed graphs (no size parameter), indexed `0..n`, useful as
//! test fixtures and benchmarks for the exact solvers: the Petersen graph,
//! the Wagner graph, the cube, the octahedron, and the small "toy" graphs of
//! five or fewer vertices. All of them are simple (no multi-edges, no
//! self-loops) and uncolored (`Color::NC` on every vertex).

use crate::generator::named::{build_with_n_vertices, hypercube, mobius_ladder};
use crate::graph::Graph;

/// The **Petersen graph**: 10 vertices, 15 edges, 3-regular. It is the
/// Kneser graph `KG(5, 2)` — vertices are the 2-subsets of a 5-element set,
/// adjacent when disjoint — but here it is drawn in the classic layout: an
/// outer 5-cycle, an inner pentagram, and five spokes. It is the standard
/// example of a graph that is not planar (it contains a `K_5` subdivision)
/// and it has treewidth 4.
pub fn petersen() -> Graph {
    let mut edges: Vec<(usize, usize)> = Vec::new();
    // Outer 5-cycle 0-1-2-3-4-0.
    for i in 0..5 {
        edges.push((i, (i + 1) % 5));
    }
    // Inner pentagram 5-7-9-6-8-5.
    for (a, b) in [(5, 7), (7, 9), (9, 6), (6, 8), (8, 5)] {
        edges.push((a, b));
    }
    // Spokes i - (5+i).
    for i in 0..5 {
        edges.push((i, 5 + i));
    }
    build_with_n_vertices(10, edges)
}

/// The **Wagner graph** (Möbius–Kantor ladder `M_4`): 8 vertices, 12 edges,
/// 3-regular. It is the Möbius ladder `mobius_ladder(4)` — a cycle of
/// length 8 with each vertex joined to the one opposite. Along with `K_5`,
/// the octahedron and the 5-prism it is one of the four minor-minimal
/// graphs of treewidth 4, so its treewidth is 4.
pub fn wagner() -> Graph {
    mobius_ladder(4)
}

/// The **cubical graph** (the 3-dimensional hypercube `Q_3`, the cube): 8
/// vertices, 12 edges, 3-regular, treewidth 3. Same graph as
/// `hypercube(3)`.
pub fn cubical() -> Graph {
    hypercube(3)
}

/// The **octahedron graph**: 6 vertices, 12 edges, 4-regular — the complete
/// graph `K_6` with a perfect matching removed. Treewidth 4.
pub fn octahedron() -> Graph {
    let mut edges = Vec::new();
    for i in 0..6 {
        for j in (i + 1)..6 {
            if !(i == 0 && j == 1) && !(i == 2 && j == 3) && !(i == 4 && j == 5) {
                edges.push((i, j));
            }
        }
    }
    build_with_n_vertices(6, edges)
}

/// The **bull graph**: 5 vertices, 5 edges — a triangle with two pendant
/// edges attached at two distinct triangle vertices. Treewidth 2.
pub fn bull() -> Graph {
    build_with_n_vertices(5, [(0, 1), (1, 2), (0, 2), (1, 3), (2, 4)])
}

/// The **house graph**: 5 vertices, 6 edges — a 4-cycle with a roof (vertex
/// 4) on top of one side. Treewidth 2.
pub fn house() -> Graph {
    build_with_n_vertices(5, [(0, 1), (1, 2), (2, 3), (3, 0), (1, 4), (2, 4)])
}

/// The **diamond graph**: 4 vertices, 5 edges — `K_4` with one edge
/// removed. Treewidth 2.
pub fn diamond() -> Graph {
    build_with_n_vertices(4, [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3)])
}

/// The **paw graph**: 4 vertices, 4 edges — a triangle with a pendant edge.
/// Treewidth 2.
pub fn paw() -> Graph {
    build_with_n_vertices(4, [(0, 1), (1, 2), (0, 2), (2, 3)])
}

/// The **bowtie graph** (butterfly graph): 5 vertices, 6 edges — two
/// triangles sharing exactly one vertex. Treewidth 2.
pub fn bowtie() -> Graph {
    build_with_n_vertices(5, [(0, 1), (1, 2), (0, 2), (0, 3), (3, 4), (0, 4)])
}

/// The **gem graph**: 5 vertices, 7 edges — `K_4` with a pendant vertex
/// attached to one of its vertices. Treewidth 3 (it contains `K_4`).
pub fn gem() -> Graph {
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for i in 0..4 {
        for j in (i + 1)..4 {
            edges.push((i, j));
        }
    }
    edges.push((0, 4));
    build_with_n_vertices(5, edges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::bb;
    use petgraph::graph::NodeIndex;

    /// Degree sequence of every vertex, in index order.
    fn degrees(g: &Graph) -> Vec<usize> {
        (0..g.node_count())
            .map(|i| g.degree(NodeIndex::new(i)))
            .collect()
    }

    #[test]
    fn petersen_structure_and_tw() {
        let g = petersen();
        assert_eq!(g.node_count(), 10);
        assert_eq!(g.edge_count(), 15);
        assert_eq!(degrees(&g), vec![3; 10]);
        assert_eq!(bb(&g).treewidth, 4);
    }

    #[test]
    fn wagner_structure_and_tw() {
        let g = wagner();
        assert_eq!(g.node_count(), 8);
        assert_eq!(g.edge_count(), 12);
        assert_eq!(degrees(&g), vec![3; 8]);
        assert_eq!(bb(&g).treewidth, 4); // one of the four tw-4 obstructions
    }

    #[test]
    fn cubical_structure_and_tw() {
        let g = cubical();
        assert_eq!(g.node_count(), 8);
        assert_eq!(g.edge_count(), 12);
        assert_eq!(degrees(&g), vec![3; 8]);
        assert_eq!(bb(&g).treewidth, 3);
        // Same graph as hypercube(3).
        assert_eq!(g.edge_count(), hypercube(3).edge_count());
    }

    #[test]
    fn octahedron_structure_and_tw() {
        let g = octahedron();
        assert_eq!(g.node_count(), 6);
        assert_eq!(g.edge_count(), 12);
        assert_eq!(degrees(&g), vec![4; 6]);
        assert_eq!(bb(&g).treewidth, 4);
    }

    #[test]
    fn toy_graphs_are_simple_and_have_expected_tw() {
        // (name, graph, vertices, edges, treewidth)
        let cases: [(&str, Graph, usize, usize, usize); 6] = [
            ("bull", bull(), 5, 5, 2),
            ("house", house(), 5, 6, 2),
            ("diamond", diamond(), 4, 5, 2),
            ("paw", paw(), 4, 4, 2),
            ("bowtie", bowtie(), 5, 6, 2),
            ("gem", gem(), 5, 7, 3),
        ];
        for (name, g, n, m, tw) in cases {
            assert_eq!(g.node_count(), n, "{name}: node count");
            assert_eq!(g.edge_count(), m, "{name}: edge count");
            assert_eq!(bb(&g).treewidth, tw, "{name}: treewidth");
        }
    }

    #[test]
    fn toy_graph_edges_exact() {
        // Spot-check the exact edge sets.
        assert!(bull().has_edge(NodeIndex::new(0), NodeIndex::new(2)));
        assert!(bull().has_edge(NodeIndex::new(1), NodeIndex::new(3)));
        assert!(!bull().has_edge(NodeIndex::new(3), NodeIndex::new(4)));
        assert!(house().has_edge(NodeIndex::new(1), NodeIndex::new(4)));
        assert!(house().has_edge(NodeIndex::new(3), NodeIndex::new(0)));
        assert!(!house().has_edge(NodeIndex::new(0), NodeIndex::new(4)));
        assert!(!diamond().has_edge(NodeIndex::new(2), NodeIndex::new(3)));
        assert!(diamond().has_edge(NodeIndex::new(0), NodeIndex::new(3)));
        assert!(paw().has_edge(NodeIndex::new(2), NodeIndex::new(3)));
        assert!(!paw().has_edge(NodeIndex::new(0), NodeIndex::new(3)));
        assert!(bowtie().has_edge(NodeIndex::new(0), NodeIndex::new(3)));
        assert!(!bowtie().has_edge(NodeIndex::new(2), NodeIndex::new(4)));
        assert!(gem().has_edge(NodeIndex::new(0), NodeIndex::new(4)));
        assert!(!gem().has_edge(NodeIndex::new(3), NodeIndex::new(4)));
    }
}
