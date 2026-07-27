//! Graph data structure for QuickBB.
//!
//! Built on top of [`petgraph::graph::UnGraph`]. Vertices are *never removed*
//! from the underlying petgraph graph; instead we keep an `alive` mask. This
//! keeps `NodeIndex` values stable across `elim` operations and lets
//! `petgraph`'s adjacency iteration continue to work after vertices have been
//! logically eliminated.
//!
//! The single most-used primitive in QuickBB is [`Graph::elim`]: make a vertex
//! simplicial (fill in the clique on its neighborhood) and remove it from the
//! graph. See §2 ("The elimination operation") of Gogate & Dechter, UAI 2004.

use fixedbitset::FixedBitSet;
use petgraph::graph::{NodeIndex, UnGraph};

/// A simple undirected graph with logical vertex deletion.
///
/// Vertices are identified by stable `NodeIndex` values (wrappers around a
/// `u32`). The graph stores no node or edge weights.
#[derive(Clone, Debug)]
pub struct Graph {
    inner: UnGraph<(), ()>,
    /// `alive[i]` is true iff vertex `i` has not been eliminated.
    alive: FixedBitSet,
}

impl Graph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Graph {
            inner: UnGraph::default(),
            alive: FixedBitSet::new(),
        }
    }

    /// Create a graph with `n` isolated vertices.
    pub fn with_capacity(n: usize) -> Self {
        let mut g = Graph::new();
        for _ in 0..n {
            g.add_vertex();
        }
        g
    }

    /// Build a graph from an edge iterator. Vertices `0..=max_index` are
    /// created on demand; edges are undirected.
    pub fn from_edges<I>(edges: I) -> Self
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        let mut g = Graph::new();
        for (u, v) in edges {
            // Ensure both endpoints exist.
            while g.alive.len() <= u.max(v) {
                g.add_vertex();
            }
            if u != v {
                g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
            }
        }
        g
    }

    /// Add a new isolated vertex and return its index.
    pub fn add_vertex(&mut self) -> NodeIndex {
        let idx = self.inner.add_node(());
        let new_len = idx.index() + 1;
        if self.alive.len() < new_len {
            self.alive.grow(new_len);
        }
        self.alive.insert(idx.index());
        idx
    }

    /// Number of vertices that currently exist in the graph (alive or not).
    pub fn node_count(&self) -> usize {
        self.inner.node_count()
    }

    /// Mark `v` as eliminated without doing any fill-in. Used by minor-min-width
    /// to drop isolated vertices.
    pub fn mark_dead(&mut self, v: NodeIndex) {
        self.alive.set(v.index(), false);
    }

    /// Number of alive (non-eliminated) vertices.
    pub fn alive_count(&self) -> usize {
        self.alive.count_ones(..)
    }

    /// Is `v` still alive (not yet eliminated)?
    pub fn is_alive(&self, v: NodeIndex) -> bool {
        self.alive.contains(v.index())
    }

    /// Iterate over all alive vertices, in ascending index order.
    pub fn alive_vertices(&self) -> impl Iterator<Item = NodeIndex> + '_ {
        self.alive.ones().map(NodeIndex::new)
    }

    /// Iterate over the alive neighbors of `v` (snapshot, ascending index).
    pub fn alive_neighbors(&self, v: NodeIndex) -> Vec<NodeIndex> {
        self.inner
            .neighbors(v)
            .filter(|&u| self.alive.contains(u.index()))
            .collect()
    }

    /// Degree of `v`: number of alive neighbors.
    pub fn degree(&self, v: NodeIndex) -> usize {
        self.alive_neighbors(v).len()
    }

    /// Is there an edge between `a` and `b`?
    pub fn has_edge(&self, a: NodeIndex, b: NodeIndex) -> bool {
        self.inner.contains_edge(a, b)
    }

    /// Add an undirected edge `{a, b}` if it does not already exist.
    /// `update_edge` replaces any existing edge rather than creating a
    /// duplicate, which is what we want for fill-in edges.
    pub fn add_edge(&mut self, a: NodeIndex, b: NodeIndex) {
        if a != b {
            self.inner.update_edge(a, b, ());
        }
    }

    /// Number of fill-in edges that would be added if `v` were eliminated:
    /// the number of non-edges among `v`'s alive neighbors. Computed as
    /// `C(deg, 2) - (existing edges among neighbors)`. Cost: O(deg^2).
    pub fn fill_in_count(&self, v: NodeIndex) -> usize {
        let nbrs = self.alive_neighbors(v);
        let d = nbrs.len();
        if d < 2 {
            return 0;
        }
        let pairs = d * (d - 1) / 2;
        // Count existing edges among the alive neighbors.
        let mut existing = 0usize;
        for i in 0..d {
            for j in (i + 1)..d {
                if self.inner.contains_edge(nbrs[i], nbrs[j]) {
                    existing += 1;
                }
            }
        }
        pairs - existing
    }

    /// Eliminate vertex `v`: connect all pairs of `v`'s alive neighbors to
    /// form a clique, then mark `v` as deleted.
    ///
    /// Returns the *degree* of `v` at elimination time (i.e. the number of
    /// alive neighbors), which the caller needs for `g(s) = max(g, degree)`.
    pub fn elim(&mut self, v: NodeIndex) -> usize {
        let nbrs = self.alive_neighbors(v);
        let d = nbrs.len();
        // Fill in the clique on the neighborhood.
        for i in 0..d {
            for j in (i + 1)..d {
                self.add_edge(nbrs[i], nbrs[j]);
            }
        }
        // Remove v.
        self.alive.set(v.index(), false);
        d
    }

    /// Contract edge `{u, v}` into a single vertex kept at `keep`. The vertex
    /// `drop` is removed; all its alive neighbors (other than `keep`) become
    /// neighbors of `keep`. Used by minor-min-width.
    pub fn contract(&mut self, keep: NodeIndex, drop: NodeIndex) {
        let drop_nbrs = self.alive_neighbors(drop);
        for x in drop_nbrs {
            if x != keep {
                self.add_edge(keep, x);
            }
        }
        self.alive.set(drop.index(), false);
    }
}

impl Default for Graph {
    fn default() -> Self {
        Graph::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g_path(n: usize) -> Graph {
        // Path 0 - 1 - 2 - ... - (n-1).
        let edges: Vec<(usize, usize)> = (0..n - 1).map(|i| (i, i + 1)).collect();
        Graph::from_edges(edges)
    }

    #[test]
    fn from_edges_and_counts() {
        // K3 triangle.
        let g = Graph::from_edges([(0, 1), (1, 2), (0, 2)]);
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.alive_count(), 3);
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(2)));
        // Self-loops are ignored.
        let g2 = Graph::from_edges([(0, 0), (0, 1)]);
        assert_eq!(g2.node_count(), 2);
        assert!(!g2.has_edge(NodeIndex::new(0), NodeIndex::new(0)));
    }

    #[test]
    fn degree_and_neighbors() {
        let g = g_path(4); // 0-1-2-3
        assert_eq!(g.degree(NodeIndex::new(0)), 1);
        assert_eq!(g.degree(NodeIndex::new(1)), 2);
        assert_eq!(g.degree(NodeIndex::new(3)), 1);
    }

    #[test]
    fn elim_fills_clique_and_removes_vertex() {
        // Star: 0 connected to 1,2,3. Eliminating 0 should make {1,2,3} a clique.
        let mut g = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        let d = g.elim(NodeIndex::new(0));
        assert_eq!(d, 3);
        assert!(!g.is_alive(NodeIndex::new(0)));
        assert!(g.has_edge(NodeIndex::new(1), NodeIndex::new(2)));
        assert!(g.has_edge(NodeIndex::new(1), NodeIndex::new(3)));
        assert!(g.has_edge(NodeIndex::new(2), NodeIndex::new(3)));
        assert_eq!(g.alive_count(), 3);
    }

    #[test]
    fn fill_in_count_star() {
        // Eliminating the center of a 3-star adds 3 fill-in edges (clique on 3).
        let g = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        assert_eq!(g.fill_in_count(NodeIndex::new(0)), 3);
        // Leaves add no fill-in.
        assert_eq!(g.fill_in_count(NodeIndex::new(1)), 0);
    }

    #[test]
    fn contract_merges_neighborhoods() {
        // 0-1-2 path. Contract {0,1} keeping 0: result 0 has neighbor 2.
        let mut g = g_path(3);
        g.contract(NodeIndex::new(0), NodeIndex::new(1));
        assert!(!g.is_alive(NodeIndex::new(1)));
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(2)));
        assert_eq!(g.alive_count(), 2);
    }
}
