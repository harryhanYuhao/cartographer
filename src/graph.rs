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

use std::path::Path;

use fixedbitset::FixedBitSet;
use petgraph::graph::{EdgeIndex, NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;

/// The per-vertex color (label).
///
/// NC ("no color") marks an uncolored vertex and is the default. Z and X carry
/// a coordinate; H is a plain tag.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VColor {
    /// Z(s) is Z spider with phase s * pi / 4; therefore s is integer mode 8
    Z(u8),
    X(u8),
    H,
    #[default]
    NC,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EColor {
    H,
    #[default]
    NC,
}

/// An undirected multigraph with logical vertex deletion and a per-vertex
/// color.
///
/// Vertices are identified by stable NodeIndex values. Each vertex stores a
/// Color in the petgraph node weight; edges are unweighted.
#[derive(Clone, Debug)]
pub struct Graph {
    inner: UnGraph<VColor, EColor>,
    /// alive[i] is true iff vertex i has not been eliminated.
    alive: FixedBitSet,
}

impl Graph {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            inner: UnGraph::default(),
            alive: FixedBitSet::new(),
        }
    }

    /// Add a new isolated vertex with the given color and return its index.
    pub fn add_vertex_with(&mut self, color: VColor) -> NodeIndex {
        let idx = self.inner.add_node(color);
        let new_len = idx.index() + 1;
        if self.alive.len() < new_len {
            self.alive.grow(new_len);
        }
        self.alive.insert(idx.index());
        idx
    }

    /// Add a new isolated vertex with the default color (NC) and return its
    /// index.
    pub fn add_vertex(&mut self) -> NodeIndex {
        self.add_vertex_with(VColor::NC)
    }

    /// The color of vertex v.
    pub fn color(&self, v: NodeIndex) -> &VColor {
        self.inner.node_weight(v).expect("vertex exists")
    }

    /// Mutable access to the color of vertex v.
    pub fn color_mut(&mut self, v: NodeIndex) -> &mut VColor {
        self.inner.node_weight_mut(v).expect("vertex exists")
    }

    /// Replace the color of vertex v.
    pub fn set_color(&mut self, v: NodeIndex, color: VColor) {
        *self.color_mut(v) = color;
    }

    /// Copy of the color of vertex v.
    pub fn label(&self, v: NodeIndex) -> VColor {
        *self.color(v)
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
        let mut nbrs: Vec<NodeIndex> = self
            .inner
            .neighbors(v)
            .filter(|&u| u != v && self.alive.contains(u.index()))
            .collect();
        nbrs.sort_unstable();
        nbrs.dedup();
        nbrs
    }

    /// Degree of `v`: number of alive neighbors.
    pub fn degree(&self, v: NodeIndex) -> usize {
        self.alive_neighbors(v).len()
    }

    /// Is there at least one edge between a and b? Use edge_multiplicity to
    /// count parallel edges.
    pub fn has_edge(&self, a: NodeIndex, b: NodeIndex) -> bool {
        self.inner.contains_edge(a, b)
    }

    /// Add an undirected edge {a, b}. The graph is a multigraph, so this
    /// always inserts a new edge: parallel edges and self-loops are allowed.
    pub fn add_edge(&mut self, a: NodeIndex, b: NodeIndex) {
        self.inner.add_edge(a, b, EColor::NC);
    }

    pub fn add_edge_c(&mut self, a: NodeIndex, b: NodeIndex, c: EColor) {
        self.inner.add_edge(a, b, c);
    }

    /// Add an undirected edge {a, b} only if none already exists between a
    /// and b. Self-loops are ignored. Used by fill-in and contraction, which
    /// must not create parallel edges.
    pub fn ensure_edge(&mut self, a: NodeIndex, b: NodeIndex) {
        if a != b {
            self.inner.update_edge(a, b, EColor::NC);
        }
    }

    pub fn ensure_edge_c(&mut self, a: NodeIndex, b: NodeIndex, c: EColor) {
        if a != b {
            self.inner.update_edge(a, b, c);
        }
    }

    /// Iterate over all edges whose endpoints are both alive, as
    /// `(source, target, edge index)` triples. Parallel edges are yielded
    /// separately; the endpoints are in petgraph's stored order (unordered
    /// for an undirected graph). The edge index stays valid even if an
    /// endpoint is later logically deleted, so it can be passed to
    /// [Graph::set_edge_color].
    pub fn edges(&self) -> impl Iterator<Item = (NodeIndex, NodeIndex, EdgeIndex)> + '_ {
        self.inner.edge_references().filter_map(|er| {
            let s = er.source();
            let t = er.target();
            if self.alive.contains(s.index()) && self.alive.contains(t.index()) {
                Some((s, t, er.id()))
            } else {
                None
            }
        })
    }

    /// The color of edge `e`.
    pub fn edge_color(&self, e: EdgeIndex) -> EColor {
        self.inner.edge_weight(e).copied().expect("edge exists")
    }

    /// Replace the color of edge `e`.
    pub fn set_edge_color(&mut self, e: EdgeIndex, c: EColor) {
        if let Some(w) = self.inner.edge_weight_mut(e) {
            *w = c;
        }
    }

    /// Number of parallel edges between a and b (0 if none). Self-loops are
    /// counted.
    pub fn edge_multiplicity(&self, a: NodeIndex, b: NodeIndex) -> usize {
        self.inner.edges_connecting(a, b).count()
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

    /// Is `v` simplicial: do its alive neighbors form a clique?
    ///
    /// A vertex with zero or one alive neighbor is vacuously simplicial.
    pub fn is_simplicial(&self, v: NodeIndex) -> bool {
        let nbrs = self.alive_neighbors(v);
        for i in 0..nbrs.len() {
            for j in (i + 1)..nbrs.len() {
                if !self.inner.contains_edge(nbrs[i], nbrs[j]) {
                    return false;
                }
            }
        }
        true
    }

    /// Is `v` almost simplicial: is there some neighbor `w` such that
    /// `N(v) \\ {w}` is a clique?
    ///
    /// This is the definition used by the graph-reduction rule of §5.1 of
    /// Gogate & Dechter (UAI 2004); a simplicial vertex is also almost
    /// simplicial.
    pub fn is_almost_simplicial(&self, v: NodeIndex) -> bool {
        let nbrs = self.alive_neighbors(v);
        if self.is_simplicial(v) {
            return true;
        }
        // Try removing each neighbor in turn; if the rest form a clique, then
        // "all but one" neighbors form a clique.
        for skip in 0..nbrs.len() {
            let mut ok = true;
            'outer: for i in 0..nbrs.len() {
                if i == skip {
                    continue;
                }
                for j in (i + 1)..nbrs.len() {
                    if j == skip {
                        continue;
                    }
                    if !self.inner.contains_edge(nbrs[i], nbrs[j]) {
                        ok = false;
                        break 'outer;
                    }
                }
            }
            if ok {
                return true;
            }
        }
        false
    }

    /// The fill-in edges that eliminating `v` would add: the non-edges among
    /// `v`'s alive neighbors, as an unordered list of vertex pairs.
    ///
    /// Used by the Theorem 6.4 dominance pruning rule.
    pub fn fill_in_edges(&self, v: NodeIndex) -> Vec<(NodeIndex, NodeIndex)> {
        let nbrs = self.alive_neighbors(v);
        let mut out = Vec::new();
        for i in 0..nbrs.len() {
            for j in (i + 1)..nbrs.len() {
                if !self.inner.contains_edge(nbrs[i], nbrs[j]) {
                    out.push((nbrs[i], nbrs[j]));
                }
            }
        }
        out
    }

    /// Number of alive vertices adjacent to both `u` and `v`.
    ///
    /// Used by the Theorem 5.4 edge-addition rule.
    pub fn common_neighbors(&self, u: NodeIndex, v: NodeIndex) -> usize {
        let mut count = 0usize;
        for x in self.alive_neighbors(u) {
            if x != v && self.inner.contains_edge(x, v) {
                count += 1;
            }
        }
        count
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
                self.ensure_edge(nbrs[i], nbrs[j]);
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
                self.ensure_edge(keep, x);
            }
        }
        self.alive.set(drop.index(), false);
    }

    pub fn remove_edge(&mut self, a: NodeIndex, b: NodeIndex) {
        if let Some(edge) = self.inner.find_edge(a, b) {
            self.inner.remove_edge(edge);
        }
    }

    pub fn remove_vertex(&mut self, v: NodeIndex) {
        self.alive.set(v.index(), false);
    }

    /// Number of edges, counting parallel edges and self-loops separately.
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Export the alive induced subgraph as Graph3 (.graph3) text.
    ///
    /// Delegates to [`crate::io::graph3::to_graph3`]: one `a : TYPE` line
    /// per alive vertex (dense labels 0..n in hex), then sorted (pair, type)
    /// edge groups. The outut is canonical, so parsing it again yields an
    /// equivalent graph.
    pub fn to_graph3(&self) -> String {
        crate::io::graph3::to_graph3(self)
    }

    /// Write this graph to a Graph3 file (see to_graph3).
    pub fn to_graph3_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        crate::io::graph3::to_graph3_file(self, path)
    }

    /// Create a graph with n isolated vertices (all NC-colored).
    pub fn with_capacity(n: usize) -> Self {
        let mut g = Self::new();
        for _ in 0..n {
            g.add_vertex();
        }
        g
    }

    /// Build an uncolored graph from an edge iterator. Vertices 0..=max_index
    /// are created on demand; duplicate pairs become parallel edges and
    /// self-loops are kept (the graph is a multigraph).
    pub fn from_edges<I>(edges: I) -> Self
    where
        I: IntoIterator<Item = (usize, usize)>,
    {
        let mut g = Self::new();
        for (u, v) in edges {
            // Ensure both endpoints exist.
            while g.alive.len() <= u.max(v) {
                g.add_vertex();
            }
            g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
        }
        g
    }

    /// Parse Graph3 (.graph3) text into a colored multigraph.
    ///
    /// Delegates to [`crate::io::graph3::from_graph3`]; see that module for
    /// the full format specification. On error the returned string carries
    /// the 1-based line number of the first offending line.
    pub fn from_graph3(input: &str) -> Result<Self, String> {
        crate::io::graph3::from_graph3(input)
    }

    /// Read a Graph3 file and parse it (see from_graph3).
    pub fn from_graph3_file(path: impl AsRef<Path>) -> Result<Self, String> {
        crate::io::graph3::from_graph3_file(path)
    }

    pub fn info(&self) -> String {
        format!(
            "Graph with {} vertices and {} edges",
            self.node_count(),
            self.edge_count()
        )
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
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
        // Self-loops are now kept.
        let g2 = Graph::from_edges([(0, 0), (0, 1)]);
        assert_eq!(g2.node_count(), 2);
        assert!(g2.has_edge(NodeIndex::new(0), NodeIndex::new(0)));
        assert_eq!(
            g2.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(0)),
            1
        );
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

    #[test]
    fn multigraph_parallel_edges_and_multiplicity() {
        let mut g = Graph::with_capacity(2);
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(0));
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(0)), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(0)), 1);
        assert_eq!(g.edge_count(), 4);
        // Distinct neighbors collapse parallel edges and ignore self-loops.
        assert_eq!(g.degree(NodeIndex::new(0)), 1);
    }

    #[test]
    fn ensure_edge_does_not_create_parallel_edges() {
        let mut g = Graph::with_capacity(2);
        g.ensure_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.ensure_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.ensure_edge(NodeIndex::new(0), NodeIndex::new(0)); // self-loop ignored
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
        assert_eq!(g.edge_count(), 1);
        assert!(!g.has_edge(NodeIndex::new(0), NodeIndex::new(0)));
    }

    #[test]
    fn alive_neighbors_dedupes_and_sorts() {
        let mut g = Graph::with_capacity(4);
        // Insert in reverse order with a duplicate and a self-loop.
        g.add_edge(NodeIndex::new(0), NodeIndex::new(3));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(2));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(2));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(0));
        assert_eq!(
            g.alive_neighbors(NodeIndex::new(0)),
            vec![NodeIndex::new(1), NodeIndex::new(2), NodeIndex::new(3)]
        );
        assert_eq!(g.degree(NodeIndex::new(0)), 3);
    }

    #[test]
    fn from_edges_keeps_parallel_edges() {
        let g = Graph::from_edges([(0, 1), (0, 1), (0, 1)]);
        assert_eq!(g.node_count(), 2);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
        assert_eq!(g.edge_count(), 3);
    }

    #[test]
    fn treewidth_ignores_parallel_edges_and_self_loops() {
        use crate::algorithm::bb_tw;
        // P5 plus parallel edges and self-loops: treewidth is still that of P5.
        let simple = Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)]);
        let multi = Graph::from_graph3(
            r"0 1 3
1 2
2 3 2
3 4
4 4 f
0 0
",
        )
        .unwrap();
        assert_eq!(bb_tw(&simple), 1);
        assert_eq!(bb_tw(&multi), 1);
    }

    #[test]
    fn colored_graph_with_enum_labels() {
        use crate::algorithm::bb_tw;

        // Build a colored graph vertex-by-vertex.
        let mut g = Graph::new();
        let a = g.add_vertex_with(VColor::Z(0));
        let b = g.add_vertex_with(VColor::X(1));
        let c = g.add_vertex_with(VColor::H);
        g.add_edge(a, b);
        g.add_edge(b, c);

        assert_eq!(g.label(a), VColor::Z(0));
        assert_eq!(g.label(b), VColor::X(1));
        assert_eq!(g.label(c), VColor::H);

        // Mutate a color in place.
        g.set_color(c, VColor::Z(2));
        assert_eq!(g.label(c), VColor::Z(2));

        // Topology and solvers work on the colored graph unchanged.
        assert_eq!(g.node_count(), 3);
        assert_eq!(bb_tw(&g), 1); // path a-b-c

        // Uncolored constructors default every vertex to NC.
        let g2 = Graph::from_edges([(0, 1), (1, 2)]);
        assert_eq!(g2.label(NodeIndex::new(0)), VColor::NC);
        assert_eq!(g2.label(NodeIndex::new(1)), VColor::NC);

        let mut g3 = Graph::new();
        g3.add_vertex();
        assert_eq!(g3.label(NodeIndex::new(0)), VColor::NC);
    }
}
