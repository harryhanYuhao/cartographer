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
pub(crate) fn build_with_n_vertices(n: usize, edges: impl IntoIterator<Item = (usize, usize)>) -> Graph {
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

/// The complete bipartite graph `K_{a,b}`: vertices `0..a` in the left
/// part, vertices `a..a+b` in the right part, and every possible edge
/// between the two parts. Treewidth `min(a, b)` when both parts are
/// non-empty; with one part empty the graph is just isolated vertices.
pub fn complete_bipartite(a: usize, b: usize) -> Graph {
    let mut edges = Vec::new();
    for i in 0..a {
        for j in 0..b {
            edges.push((i, a + j));
        }
    }
    build_with_n_vertices(a + b, edges)
}

/// The wheel graph `W_n`: a cycle on rim vertices `0..n` with a hub vertex
/// `n` adjacent to every rim vertex. `n == 3` is `K_4`; for smaller `n`
/// the rim cycle is dropped and only the spokes remain (a star). Treewidth
/// 3 for `n >= 4`.
pub fn wheel(n: usize) -> Graph {
    let mut edges: Vec<(usize, usize)> = (0..n).map(|i| (i, n)).collect();
    if n >= 3 {
        for i in 0..n {
            edges.push((i, (i + 1) % n));
        }
    }
    build_with_n_vertices(n + 1, edges)
}

/// The `d`-dimensional hypercube graph `Q_d`: `2^d` vertices indexed by
/// the `d`-bit numbers, with an edge between vertices whose labels differ
/// in exactly one bit. `Q_0` is a single vertex; `Q_3` (the cube, 8
/// vertices) has treewidth 3. For `d` so large that `2^d` would overflow
/// `usize`, an empty graph is returned.
pub fn hypercube(d: usize) -> Graph {
    let n = 1usize.checked_shl(d as u32).unwrap_or(0);
    let mut edges = Vec::new();
    for u in 0..n {
        for v in (u + 1)..n {
            if (u ^ v).is_power_of_two() {
                edges.push((u, v));
            }
        }
    }
    build_with_n_vertices(n, edges)
}

/// The Möbius ladder `M_n` on `2n` vertices: a cycle `0..2n` with a chord
/// `{i, i+n}` for each `0 <= i < n`. Requires `n >= 3`; smaller `n` yields
/// the bare cycle. The graph is 3-regular. `M_3` is the triangular prism
/// (treewidth 3) and `M_4` is the Wagner graph (treewidth 4).
pub fn mobius_ladder(n: usize) -> Graph {
    let mut edges: Vec<(usize, usize)> = (0..2 * n).map(|i| (i, (i + 1) % (2 * n))).collect();
    if n >= 3 {
        for i in 0..n {
            edges.push((i, i + n));
        }
    }
    build_with_n_vertices(2 * n, edges)
}

/// The `n`-prism graph `Y_n = C_n □ K_2` on `2n` vertices: an outer cycle
/// on `0..n`, an inner cycle on `n..2n`, and rung edges `{i, n+i}`. The
/// triangular prism (`n = 3`, 6 vertices) and the cube (`n = 4`, 8
/// vertices) have treewidth 3; the pentagonal prism (`n = 5`, 10 vertices)
/// has treewidth 4 (it is one of the four minor-minimal graphs of
/// treewidth 4). Requires `n >= 3`; smaller `n` yields just the rungs.
pub fn prism(n: usize) -> Graph {
    let n2 = 2 * n;
    let mut edges: Vec<(usize, usize)> = (0..n).map(|i| (i, n + i)).collect();
    if n >= 3 {
        for i in 0..n {
            edges.push((i, (i + 1) % n));
            edges.push((n + i, n + (i + 1) % n));
        }
    }
    build_with_n_vertices(n2, edges)
}

/// The friendship (windmill) graph `F_n`: `n` triangles sharing the single
/// common vertex `0`, for `2n + 1` vertices in total. Treewidth 2 for
/// `n >= 1`; `F_0` is a single isolated vertex.
pub fn friendship(n: usize) -> Graph {
    let mut edges = Vec::new();
    for k in 0..n {
        let a = 2 * k + 1;
        let b = 2 * k + 2;
        edges.push((0, a));
        edges.push((0, b));
        edges.push((a, b));
    }
    build_with_n_vertices(2 * n + 1, edges)
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

    #[test]
    fn complete_bipartite_k33() {
        let g = complete_bipartite(3, 3);
        assert_eq!(g.node_count(), 6);
        assert_eq!(g.edge_count(), 9);
        for i in 0..3 {
            for j in 0..3 {
                assert!(g.has_edge(NodeIndex::new(i), NodeIndex::new(3 + j)));
            }
            assert!(!g.has_edge(NodeIndex::new(i), NodeIndex::new((i + 1) % 3)));
        }
        assert_eq!(bb(&g).treewidth, 3);
        // One part empty: isolated vertices, treewidth 0.
        let g2 = complete_bipartite(4, 0);
        assert_eq!(g2.node_count(), 4);
        assert_eq!(g2.edge_count(), 0);
        assert_eq!(bb(&g2).treewidth, 0);
    }

    #[test]
    fn wheel_w6() {
        let g = wheel(6);
        assert_eq!(g.node_count(), 7);
        assert_eq!(g.edge_count(), 12); // 6 spokes + 6 rim edges
        // Hub 6 adjacent to every rim vertex.
        for i in 0..6 {
            assert!(g.has_edge(NodeIndex::new(i), NodeIndex::new(6)));
        }
        assert_eq!(g.degree(NodeIndex::new(6)), 6);
        assert_eq!(bb(&g).treewidth, 3);
        // W_3 = K_4.
        assert_eq!(wheel(3).edge_count(), 6);
    }

    #[test]
    fn hypercube_q3() {
        let g = hypercube(3);
        assert_eq!(g.node_count(), 8);
        assert_eq!(g.edge_count(), 12); // 8 * 3 / 2
        for v in 0..8 {
            assert_eq!(g.degree(NodeIndex::new(v)), 3, "vertex {v}");
        }
        assert_eq!(bb(&g).treewidth, 3);
        // Q_0, Q_1, Q_2 sanity.
        assert_eq!(hypercube(0).node_count(), 1);
        assert_eq!(hypercube(1).edge_count(), 1);
        assert_eq!(hypercube(2).edge_count(), 4);
        // Hamming-distance rule spot check: 0-7 differ in 3 bits, no edge.
        assert!(!g.has_edge(NodeIndex::new(0), NodeIndex::new(7)));
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(4)));
    }

    #[test]
    fn mobius_ladder_m4_is_wagner() {
        let g = mobius_ladder(4);
        assert_eq!(g.node_count(), 8);
        assert_eq!(g.edge_count(), 12);
        for v in 0..8 {
            assert_eq!(g.degree(NodeIndex::new(v)), 3, "vertex {v}");
        }
        assert_eq!(bb(&g).treewidth, 4); // the Wagner graph
        // Chords join opposite vertices.
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(4)));
        assert!(g.has_edge(NodeIndex::new(3), NodeIndex::new(7)));
    }

    #[test]
    fn prisms() {
        // Triangular prism: 6 vertices, 9 edges, treewidth 3.
        let t = prism(3);
        assert_eq!(t.node_count(), 6);
        assert_eq!(t.edge_count(), 9);
        for v in 0..6 {
            assert_eq!(t.degree(NodeIndex::new(v)), 3, "vertex {v}");
        }
        assert_eq!(bb(&t).treewidth, 3);
        // Pentagonal prism: 10 vertices, 15 edges, treewidth 4.
        let p = prism(5);
        assert_eq!(p.node_count(), 10);
        assert_eq!(p.edge_count(), 15);
        assert_eq!(bb(&p).treewidth, 4);
        // Rungs connect the two cycles.
        assert!(p.has_edge(NodeIndex::new(0), NodeIndex::new(5)));
        assert!(p.has_edge(NodeIndex::new(9), NodeIndex::new(4)));
    }

    #[test]
    fn friendship_f3() {
        let g = friendship(3);
        assert_eq!(g.node_count(), 7);
        assert_eq!(g.edge_count(), 9);
        assert_eq!(g.degree(NodeIndex::new(0)), 6);
        assert_eq!(bb(&g).treewidth, 2);
        // Every other vertex has degree 2.
        for v in 1..7 {
            assert_eq!(g.degree(NodeIndex::new(v)), 2, "vertex {v}");
        }
    }
}
