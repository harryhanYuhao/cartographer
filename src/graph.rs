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

use std::collections::HashMap;
use std::path::Path;

use fixedbitset::FixedBitSet;
use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::EdgeRef;

/// The per-vertex color (label).
///
/// NC ("no color") marks an uncolored vertex and is the default. Z and X carry
/// a coordinate; H is a plain tag.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum VColor {
    Z(f64),
    X(f64),
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
    inner: UnGraph<VColor, (EColor)>,
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
    /// Vertex labels are the dense alive indices 0..n rendered in hex.
    /// Parallel edges and self-loops are preserved. The output is canonical,
    /// so parsing it again yields an equivalent graph.
    pub fn to_graph3(&self) -> String {
        let alive: Vec<NodeIndex> = self.alive_vertices().collect();
        let mut remap: HashMap<usize, usize> = HashMap::new();
        for (i, &v) in alive.iter().enumerate() {
            remap.insert(v.index(), i);
        }

        // Count parallel edges (including self-loops) among alive endpoints.
        let mut mult: HashMap<(usize, usize), u32> = HashMap::new();
        for er in self.inner.edge_references() {
            let s = er.source().index();
            let t = er.target().index();
            if !self.alive.contains(s) || !self.alive.contains(t) {
                continue;
            }
            let key = if s <= t { (s, t) } else { (t, s) };
            *mult.entry(key).or_insert(0) += 1;
        }

        let mut edges: Vec<(usize, usize, u32)> = mult
            .into_iter()
            .map(|((s, t), m)| {
                let mut a = remap[&s];
                let mut b = remap[&t];
                if a > b {
                    std::mem::swap(&mut a, &mut b);
                }
                (a, b, m)
            })
            .collect();
        edges.sort_by_key(|&(a, b, _)| (a, b));

        let mut out = String::new();
        let mut incident = vec![false; alive.len()];
        for &(a, b, _) in &edges {
            incident[a] = true;
            incident[b] = true;
        }
        for (i, _) in alive.iter().enumerate() {
            if !incident[i] {
                out.push_str(&format!("{i:x}\n"));
            }
        }
        for (a, b, m) in edges {
            if m == 1 {
                out.push_str(&format!("{a:x} {b:x}\n"));
            } else {
                out.push_str(&format!("{a:x} {b:x} {m:x}\n"));
            }
        }
        out
    }

    /// Write this graph to a Graph3 file (see to_graph3).
    pub fn to_graph3_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::write(path.as_ref(), self.to_graph3())
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

    /// Parse Graph3 (.graph3) text into an uncolored multigraph.
    ///
    /// Graph3 is a line-based format in which each non-blank line holds 1, 2,
    /// or 3 bare hex tokens. A lone token declares an isolated vertex, two
    /// tokens declare one edge, and three tokens declare the third as a
    /// parallel-edge count. Labels are compared case-insensitively and with
    /// leading zeros ignored, edge endpoints are unordered, and when a pair is
    /// repeated only the last line counts. Self-loops and a zero multiplicity
    /// (which declares its endpoints without an edge) are allowed.
    ///
    /// Labels are remapped to dense vertex indices 0..n in ascending numeric
    /// order. On error the returned string carries the 1-based line number of
    /// the first offending line (4 or more tokens, a non-hex token, or a
    /// multiplicity greater than 10000).
    pub fn from_graph3(input: &str) -> Result<Self, String> {
        let mut labels: Vec<String> = Vec::new();
        let mut ids: HashMap<String, usize> = HashMap::new();
        let mut edge_map: HashMap<(usize, usize), u32> = HashMap::new();

        for (i, raw) in input.lines().enumerate() {
            let line_no = i + 1;
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let n = tokens.len();
            if n == 0 || n > 3 {
                return Err(format!(
                    "graph3 line {line_no}: expected 1, 2, or 3 tokens, found {n}"
                ));
            }

            let a = canonical_hex(tokens[0], line_no)?;
            let ia = register_label(&mut labels, &mut ids, a);
            let ib = if n >= 2 {
                let b = canonical_hex(tokens[1], line_no)?;
                Some(register_label(&mut labels, &mut ids, b))
            } else {
                None
            };

            match n {
                1 => {}
                2 => {
                    edge_map.insert(order_pair(ia, ib.unwrap()), 1);
                }
                3 => {
                    let m = graph3_multiplicity(tokens[2], line_no)?;
                    if m > 0 {
                        edge_map.insert(order_pair(ia, ib.unwrap()), m);
                    }
                }
                _ => unreachable!(),
            }
        }

        // Assign dense ids in ascending numeric-label order.
        let mut order: Vec<usize> = (0..labels.len()).collect();
        order.sort_by_key(|&i| numeric_label_key(&labels[i]));
        let mut remap = vec![0usize; labels.len()];
        for (new, &old) in order.iter().enumerate() {
            remap[old] = new;
        }

        let mut edges: Vec<(usize, usize, u32)> = edge_map
            .into_iter()
            .map(|((a, b), m)| {
                let mut x = remap[a];
                let mut y = remap[b];
                if x > y {
                    std::mem::swap(&mut x, &mut y);
                }
                (x, y, m)
            })
            .collect();
        edges.sort_by_key(|&(a, b, _)| (a, b));

        let mut g = Self::with_capacity(labels.len());
        for (a, b, m) in edges {
            for _ in 0..m {
                g.add_edge(NodeIndex::new(a), NodeIndex::new(b));
            }
        }
        Ok(g)
    }

    /// Read a Graph3 file and parse it (see from_graph3).
    pub fn from_graph3_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = std::fs::read_to_string(path.as_ref())
            .map_err(|e| format!("failed to read graph3 file: {e}"))?;
        Self::from_graph3(&text)
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}

/// Validate a Graph3 token is bare hex and return its canonical (lowercase,
/// no leading zeros) form.
fn canonical_hex(token: &str, line: usize) -> Result<String, String> {
    if token.is_empty() || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("graph3 line {line}: invalid hex token '{token}'"));
    }
    let lower = token.to_ascii_lowercase();
    let trimmed = lower.trim_start_matches('0');
    Ok(if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    })
}

/// Parse a Graph3 multiplicity token, enforcing the 10 000 ceiling.
fn graph3_multiplicity(token: &str, line: usize) -> Result<u32, String> {
    let canon = canonical_hex(token, line)?;
    if canon.len() > 4 {
        return Err(format!(
            "graph3 line {line}: multiplicity '{token}' exceeds 10000"
        ));
    }
    let m = u32::from_str_radix(&canon, 16)
        .map_err(|_| format!("graph3 line {line}: invalid multiplicity '{token}'"))?;
    if m > 10_000 {
        return Err(format!(
            "graph3 line {line}: multiplicity '{token}' exceeds 10000"
        ));
    }
    Ok(m)
}

/// Intern a canonical label, returning its vertex id (first-appearance order).
fn register_label(
    labels: &mut Vec<String>,
    ids: &mut HashMap<String, usize>,
    label: String,
) -> usize {
    if let Some(&i) = ids.get(&label) {
        return i;
    }
    let i = labels.len();
    labels.push(label.clone());
    ids.insert(label, i);
    i
}

/// Canonical unordered pair key.
fn order_pair(a: usize, b: usize) -> (usize, usize) {
    if a <= b { (a, b) } else { (b, a) }
}

/// Numeric sort key for a canonical label: shorter (fewer hex digits) is
/// smaller; equal length compares lexicographically.
fn numeric_label_key(label: &str) -> (usize, &str) {
    (label.len(), label)
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
    fn graph3_import_export_round_trip() {
        let src = r"1 2 2
2 3
1
1 3
a a f
10 20 3
";
        let g = Graph::from_graph3(src).unwrap();
        assert_eq!(g.node_count(), 6);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 2); // 1-2
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 1); // 2-3
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)), 1); // 1-3
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(3), NodeIndex::new(3)),
            15
        ); // a-a
        assert_eq!(g.edge_multiplicity(NodeIndex::new(4), NodeIndex::new(5)), 3); // 10-20
        let out = g.to_graph3();
        assert_eq!(out, Graph::from_graph3(&out).unwrap().to_graph3());
    }

    #[test]
    fn graph3_isolated_vertex_and_zero_multiplicity() {
        let g = Graph::from_graph3(
            r"f
1 2 0
",
        )
        .unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_count(), 0);
        let out = g.to_graph3();
        assert_eq!(out.lines().count(), 3);
        assert_eq!(Graph::from_graph3(&out).unwrap().node_count(), 3);
    }

    #[test]
    fn graph3_last_line_wins() {
        let g = Graph::from_graph3(
            r"1 2 3
2 1
",
        )
        .unwrap();
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
    }

    #[test]
    fn graph3_errors() {
        assert!(Graph::from_graph3(r"1 2 3 4").is_err());
        assert!(Graph::from_graph3(r"1 0x1").is_err());
        assert!(Graph::from_graph3(r"1 -1").is_err());
        assert!(Graph::from_graph3(r"1 2 2711").is_err()); // 10001 > ceiling
        assert!(Graph::from_graph3(r"1 2 2710").is_ok()); // 10000 ok
    }

    #[test]
    fn graph3_canonicalizes_labels() {
        // a, A, 0a, 00a all spell hex 10, so they are one vertex.
        let g = Graph::from_graph3(
            r"a
A
0a
00a
",
        )
        .unwrap();
        assert_eq!(g.node_count(), 1);

        // A == a and 0B == b, so the repeated pair collapses and the last line wins.
        let g2 = Graph::from_graph3(
            r"A 0B 2
00a b 3
",
        )
        .unwrap();
        assert_eq!(g2.node_count(), 2);
        assert_eq!(
            g2.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            3
        );
    }

    #[test]
    fn graph3_orders_vertices_numerically() {
        // Labels 2, f(15), 10(16). Numeric order is 2 < f < 10, so 10 is node 2
        // even though "10" sorts before "2" as text.
        let g = Graph::from_graph3(
            "10 2
f
",
        )
        .unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 0);
    }

    #[test]
    fn graph3_ignores_blank_lines_and_whitespace() {
        let g = Graph::from_graph3(
            r"
1 2
   
2 3
",
        )
        .unwrap();
        assert_eq!(g.node_count(), 3);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 1);
        assert_eq!(g.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)), 1);
    }

    #[test]
    fn graph3_handles_tabs() {
        let g = Graph::from_graph3(
            "1	2	3
",
        )
        .unwrap();
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)), 3);
    }

    #[test]
    fn graph3_self_loop_does_not_affect_degree() {
        let g = Graph::from_graph3(
            "a a
",
        )
        .unwrap();
        assert_eq!(g.node_count(), 1);
        assert!(g.has_edge(NodeIndex::new(0), NodeIndex::new(0)));
        assert_eq!(g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(0)), 1);
        // A self-loop is an edge but not a neighbor.
        assert_eq!(g.degree(NodeIndex::new(0)), 0);
        assert_eq!(g.alive_neighbors(NodeIndex::new(0)), vec![]);
    }

    #[test]
    fn graph3_multiplicity_ceiling_boundaries() {
        // 10000 (0x2710) is the largest allowed multiplicity.
        let g = Graph::from_graph3("1 2 2710").unwrap();
        assert_eq!(g.edge_count(), 10000);
        assert_eq!(
            g.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            10000
        );
        // Just over the ceiling, and values that overflow a u16.
        assert!(Graph::from_graph3("1 2 2711").is_err());
        assert!(Graph::from_graph3("1 2 ffff").is_err());
        assert!(Graph::from_graph3("1 2 100000").is_err());
        // Leading zeros are ignored, so "0000" is a zero multiplicity.
        let g0 = Graph::from_graph3("1 2 0000").unwrap();
        assert_eq!(g0.edge_count(), 0);
        assert_eq!(g0.node_count(), 2);
    }

    #[test]
    fn graph3_errors_report_line_numbers() {
        let e = Graph::from_graph3(
            r"1 2
2 3
0x4 5
",
        )
        .unwrap_err();
        assert!(e.contains("line 3"), "got: {e}");
        let e2 = Graph::from_graph3(
            "1 2

2 3 2711
",
        )
        .unwrap_err();
        assert!(e2.contains("line 3"), "got: {e2}");
    }

    #[test]
    fn graph3_empty_input() {
        assert_eq!(Graph::from_graph3("").unwrap().node_count(), 0);
        assert_eq!(
            Graph::from_graph3(
                "  

  "
            )
            .unwrap()
            .node_count(),
            0
        );
    }

    #[test]
    fn graph3_export_from_constructed_graph() {
        let mut g = Graph::with_capacity(3);
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(1));
        g.add_edge(NodeIndex::new(0), NodeIndex::new(0)); // self-loop
        // vertex 2 is isolated
        let out = g.to_graph3();
        assert_eq!(
            out,
            "2
0 0
0 1 2
"
        );
        assert_eq!(Graph::from_graph3(&out).unwrap().to_graph3(), out);
    }

    #[test]
    fn graph3_export_is_alive_induced_subgraph() {
        // Eliminate the center of a star: only leaves 1,2,3 stay alive, and
        // elimination fills them into a clique.
        let mut g = Graph::from_edges([(0, 1), (0, 2), (0, 3)]);
        g.elim(NodeIndex::new(0));
        assert_eq!(g.alive_count(), 3);
        let out = g.to_graph3();
        let parsed = Graph::from_graph3(&out).unwrap();
        assert_eq!(parsed.node_count(), 3);
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(1)),
            1
        );
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(1), NodeIndex::new(2)),
            1
        );
        assert_eq!(
            parsed.edge_multiplicity(NodeIndex::new(0), NodeIndex::new(2)),
            1
        );
    }

    #[test]
    fn graph3_file_round_trip() {
        let path =
            std::env::temp_dir().join(format!("cartographer_graph3_{}.graph3", std::process::id()));
        let g = Graph::from_graph3(
            r"1 2 3
2 3
f
",
        )
        .unwrap();
        g.to_graph3_file(&path).unwrap();
        let g2 = Graph::from_graph3_file(&path).unwrap();
        assert_eq!(g.to_graph3(), g2.to_graph3());
        std::fs::remove_file(&path).ok();
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
        use crate::bb_tw;
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
        use crate::bb_tw;

        // Build a colored graph vertex-by-vertex.
        let mut g = Graph::new();
        let a = g.add_vertex_with(VColor::Z(0.5));
        let b = g.add_vertex_with(VColor::X(1.0));
        let c = g.add_vertex_with(VColor::H);
        g.add_edge(a, b);
        g.add_edge(b, c);

        assert_eq!(g.label(a), VColor::Z(0.5));
        assert_eq!(g.label(b), VColor::X(1.0));
        assert_eq!(g.label(c), VColor::H);

        // Mutate a color in place.
        g.set_color(c, VColor::Z(2.0));
        assert_eq!(g.label(c), VColor::Z(2.0));

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
