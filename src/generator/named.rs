//! Deterministic named graphs.
//!
//! Each function builds a graph with vertices indexed `0..n` and returns it
//! as a fresh [`Graph`]. These mirror the small helpers that used to live
//! inside `#[cfg(test)]` modules across the crate.

use crate::graph::Graph;
use petgraph::graph::NodeIndex;

/// Build a graph with exactly `n` vertices `0..n` (isolated if `n > 0`) and
/// the given edges added on top.
///
/// We can't just use [`Graph::from_edges`]: that constructor only creates
/// vertices that are referenced by some edge, so edge-less requests (e.g.
/// `path(1)`, `complete_binary_tree(0)`) would otherwise come back with zero
/// vertices.
fn build_with_n_vertices(n: usize, edges: impl IntoIterator<Item = (usize, usize)>) -> Graph {
    let mut g = Graph::with_capacity(n);
    for (u, v) in edges {
        if u != v {
            g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
        }
    }
    g
}

/// The path graph `P_n`: vertices `0..n` joined by edges
/// `{i, i+1}` for `0 <= i < n`. `P_0` and `P_1` are the empty / single-vertex
/// graphs. Treewidth 1 for `n >= 2`.
pub fn path(n: usize) -> Graph {
    let edges: Vec<(usize, usize)> = (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect();
    build_with_n_vertices(n, edges)
}

/// The cycle graph `C_n`: `P_n` with an additional closing edge
/// `{n-1, 0}`. Requires `n >= 3` for a genuine cycle; smaller `n` simply
/// yields a path (no self/multi-edges are ever inserted). Treewidth 2 for
/// `n >= 3`.
pub fn cycle(n: usize) -> Graph {
    let mut edges: Vec<(usize, usize)> = (0..n.saturating_sub(1)).map(|i| (i, i + 1)).collect();
    if n >= 3 {
        edges.push((n - 1, 0));
    }
    build_with_n_vertices(n, edges)
}

/// The complete graph `K_n`: every pair of distinct vertices is adjacent.
/// Treewidth `n - 1`.
pub fn clique(n: usize) -> Graph {
    let mut edges = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            edges.push((i, j));
        }
    }
    build_with_n_vertices(n, edges)
}

/// The star graph on `n` vertices: a center vertex `0` adjacent to every
/// other vertex. Treewidth 1 for `n >= 2`.
pub fn star(n: usize) -> Graph {
    build_with_n_vertices(n, (1..n).map(|i| (0, i)))
}

/// A rectangular grid graph with `rows` rows and `cols` columns (vertices
/// laid out row-major, `index = row * cols + col`). Horizontal edges within
/// each row and vertical edges within each column. Treewidth
/// `min(rows, cols)` for grids of size at least 2×2.
pub fn grid(rows: usize, cols: usize) -> Graph {
    let n = rows * cols;
    let mut edges = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let i = r * cols + c;
            if c + 1 < cols {
                edges.push((i, i + 1));
            }
            if r + 1 < rows {
                edges.push((i, i + cols));
            }
        }
    }
    build_with_n_vertices(n, edges)
}

/// A complete binary tree of the given `depth` (number of edges from root to
/// the deepest leaf). Vertex `i` has children `2i+1` and `2i+2`; the tree has
/// `2^(depth+1) - 1` vertices. `depth == 0` yields a single root vertex.
/// Treewidth 1 for `depth >= 1`.
pub fn complete_binary_tree(depth: usize) -> Graph {
    let n = (1usize << (depth + 1)).saturating_sub(1);
    let mut edges = Vec::new();
    for i in 0..n / 2 {
        edges.push((i, 2 * i + 1));
        edges.push((i, 2 * i + 2));
    }
    build_with_n_vertices(n, edges)
}

/// `n` isolated vertices (no edges). A thin wrapper around
/// [`Graph::with_capacity`] kept for naming symmetry with the other
/// generators.
pub fn empty(n: usize) -> Graph {
    Graph::with_capacity(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::bb;
    use petgraph::graph::NodeIndex;

    #[test]
    fn path_counts_and_tw() {
        let g = path(5);
        assert_eq!(g.node_count(), 5);
        assert_eq!(g.alive_count(), 5);
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(!g.has_edge(NodeIndex::new(0), NodeIndex::new(2)));
        assert_eq!(bb(&g).treewidth, 1);
    }

    #[test]
    fn path_small() {
        assert_eq!(path(0).node_count(), 0);
        assert_eq!(path(1).node_count(), 1);
        assert_eq!(path(2).node_count(), 2);
    }

    #[test]
    fn cycle_c5() {
        let g = cycle(5);
        assert_eq!(g.node_count(), 5);
        assert!(g.has_edge(NodeIndex::new(4), NodeIndex::new(0)));
        assert_eq!(bb(&g).treewidth, 2);
    }

    #[test]
    fn clique_k4() {
        let g = clique(4);
        assert_eq!(g.node_count(), 4);
        for u in 0..4 {
            for v in (u + 1)..4 {
                assert!(g.has_edge(NodeIndex::new(u), NodeIndex::new(v)));
            }
        }
        assert_eq!(bb(&g).treewidth, 3);
    }

    #[test]
    fn star_tw_one() {
        let g = star(6);
        assert_eq!(g.node_count(), 6);
        assert_eq!(bb(&g).treewidth, 1);
    }

    #[test]
    fn grid_3x3_tw_three() {
        let g = grid(3, 3);
        assert_eq!(g.node_count(), 9);
        assert_eq!(bb(&g).treewidth, 3);
    }

    #[test]
    fn binary_tree_tw_one() {
        // depth 3 -> 15 vertices
        let g = complete_binary_tree(3);
        assert_eq!(g.node_count(), 15);
        assert_eq!(bb(&g).treewidth, 1);
    }

    #[test]
    fn binary_tree_depth_zero() {
        let g = complete_binary_tree(0);
        assert_eq!(g.node_count(), 1);
        assert_eq!(g.alive_count(), 1);
    }

    #[test]
    fn empty_graph() {
        let g = empty(4);
        assert_eq!(g.node_count(), 4);
        assert_eq!(g.alive_count(), 4);
    }
}
