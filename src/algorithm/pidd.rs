//! Tamaki's PID-BT algorithm for exact treewidth.
//!
//! Implements the positive-instance driven dynamic programming algorithm of
//! *Positive-instance driven dynamic programming for treewidth* (Tamaki,
//! ESA 2017 / arXiv:1704.05286). It decides `tw(G) <= k` by generating only
//! the *feasible* I-blocks, O-blocks and potential maximal cliques, built up by
//! binary recurrences rather than enumerating all minimal separators and
//! potential maximal cliques (the Bouchitté–Todinca scheme).
//!
//! This is the algorithm that dominates the exact tracks of the PACE 2016/2017
//! treewidth challenges and, unlike the QuickBB branch-and-bound in
//! [`super::branchbound`], scales to the large sparse instances that
//! branch-and-bound handles poorly.

use fixedbitset::FixedBitSet;
use petgraph::graph::NodeIndex;
use std::collections::HashSet;

/// A vertex set as a fixed-width bitset. All sets in a graph share the same
/// width `n` so that hashing and equality are consistent.
type Bits = FixedBitSet;

/// An undirected graph stored as adjacency bitsets (self-contained, for fast
/// set neighbourhood / component queries).
struct BGraph {
    n: usize,
    adj: Vec<Bits>,
}

impl BGraph {
    /// Build from the crate's (petgraph-backed) graph, relabelling the alive
    /// vertices densely to `0..n`.
    fn from_crate(g: &crate::graph::Graph) -> BGraph {
        let alive: Vec<usize> = g.alive_vertices().map(|x| x.index()).collect();
        let mut map = vec![usize::MAX; g.node_count()];
        for (new, &old) in alive.iter().enumerate() {
            map[old] = new;
        }
        let n = alive.len();
        let mut adj = vec![Bits::with_capacity(n); n];
        for (new, &old) in alive.iter().enumerate() {
            for nb in g.alive_neighbors(NodeIndex::new(old)) {
                adj[new].insert(map[nb.index()]);
            }
        }
        BGraph { n, adj }
    }

    /// Open neighbourhood of a vertex set: vertices outside `s` adjacent to `s`.
    fn neighborhood(&self, s: &Bits) -> Bits {
        let mut r = Bits::with_capacity(self.n);
        for v in s.ones() {
            r.union_with(&self.adj[v]);
        }
        r.difference_with(s);
        r
    }

    /// Closed neighbourhood `N[v] = {v} ∪ N(v)`.
    fn closed_neighborhood(&self, v: usize) -> Bits {
        let mut r = self.adj[v].clone();
        r.insert(v);
        r
    }

    /// Connected components of `G - removed`.
    fn components(&self, removed: &Bits) -> Vec<Bits> {
        let mut visited = Bits::with_capacity(self.n);
        let mut result = Vec::new();
        for start in 0..self.n {
            if removed.contains(start) || visited.contains(start) {
                continue;
            }
            let mut comp = Bits::with_capacity(self.n);
            let mut stack = vec![start];
            visited.insert(start);
            comp.insert(start);
            while let Some(u) = stack.pop() {
                for v in self.adj[u].ones() {
                    if removed.contains(v) || visited.contains(v) {
                        continue;
                    }
                    visited.insert(v);
                    comp.insert(v);
                    stack.push(v);
                }
            }
            result.push(comp);
        }
        result
    }

    /// Full components of `s`: components `C` of `G - s` with `N(C) = s`.
    fn full_components(&self, s: &Bits) -> Vec<Bits> {
        self.components(s)
            .into_iter()
            .filter(|c| self.neighborhood(c) == *s)
            .collect()
    }

    /// The outbound full component of `s` (the one with the smallest minimum
    /// vertex), if any.
    fn outbound_full_component(&self, s: &Bits) -> Option<Bits> {
        self.full_components(s).into_iter().min_by_key(|c| c.minimum())
    }

    /// Is `s` a potential maximal clique (Theorem 3.15 of Bouchitté–Todinca):
    /// no full component, and cliquish.
    fn is_pmc(&self, s: &Bits) -> bool {
        let comps = self.components(s);
        for c in &comps {
            if self.neighborhood(c) == *s {
                return false;
            }
        }
        // cliquish (reuse the components already computed)
        let nbrs: Vec<Bits> = comps.iter().map(|c| self.neighborhood(c)).collect();
        let vs: Vec<usize> = s.ones().collect();
        for i in 0..vs.len() {
            for j in (i + 1)..vs.len() {
                let (u, v) = (vs[i], vs[j]);
                if self.adj[u].contains(v) {
                    continue;
                }
                let mut covered = false;
                for nb in &nbrs {
                    if nb.contains(u) && nb.contains(v) {
                        covered = true;
                        break;
                    }
                }
                if !covered {
                    return false;
                }
            }
        }
        true
    }

    /// The crib of `s` with respect to `k` (both vertex sets, `s ⊂ k`,
    /// `k` cliquish): `(k \ s)` plus every component of `G - k` not confined
    /// to `s`.
    fn crib(&self, s: &Bits, k: &Bits) -> Bits {
        let mut result = k.clone();
        result.difference_with(s);
        let k_minus_s = result.clone();
        for c in self.components(k) {
            let nc = self.neighborhood(&c);
            if !nc.is_disjoint(&k_minus_s) {
                result.union_with(&c);
            }
        }
        result
    }

    /// Is component `c` outbound (no other full component of `N(c)` precedes it)?
    fn is_outbound(&self, c: &Bits) -> bool {
        let s = self.neighborhood(c);
        let m = c.minimum().expect("component is non-empty");
        for e in self.full_components(&s) {
            if let Some(em) = e.minimum() {
                if em < m {
                    return false;
                }
            }
        }
        true
    }

    /// The outlet of a cliquish set `k`: the neighbourhood of a maximal
    /// outbound non-full component of `G - k`, or the empty set if none exists.
    fn outlet(&self, k: &Bits) -> Bits {
        let mut best: Option<Bits> = None;
        for c in self.components(k) {
            let nc = self.neighborhood(&c);
            if nc == *k {
                continue; // full component
            }
            if !self.is_outbound(&c) {
                continue;
            }
            match &best {
                None => best = Some(nc),
                Some(b) => {
                    // Lemma 4: outbound non-full components have nested
                    // neighbourhoods, so keep the inclusion-maximal one.
                    if nc.is_superset(b) {
                        best = Some(nc);
                    }
                }
            }
        }
        best.unwrap_or_else(|| Bits::with_capacity(self.n))
    }

    /// Components of `G - k` not confined to `outlet(k)`.
    fn support(&self, k: &Bits) -> Vec<Bits> {
        let outlet = self.outlet(k);
        self.components(k)
            .into_iter()
            .filter(|c| !self.neighborhood(c).is_subset(&outlet))
            .collect()
    }

    /// Minimum degree (a valid lower bound on treewidth).
    fn min_degree(&self) -> usize {
        (0..self.n)
            .map(|v| self.adj[v].count_ones(..))
            .min()
            .unwrap_or(0)
    }

    /// Induced subgraph on `verts`, with vertices relabelled densely.
    fn induced_subgraph(&self, verts: &Bits) -> BGraph {
        let idx: Vec<usize> = verts.ones().collect();
        let mut map = vec![usize::MAX; self.n];
        for (new, &old) in idx.iter().enumerate() {
            map[old] = new;
        }
        let m = idx.len();
        let mut adj = vec![Bits::with_capacity(m); m];
        for (new, &old) in idx.iter().enumerate() {
            for nb in self.adj[old].ones() {
                if verts.contains(nb) {
                    adj[new].insert(map[nb]);
                }
            }
        }
        BGraph { n: m, adj }
    }

    /// Is `s` a clique?
    fn is_clique(&self, s: &Bits) -> bool {
        let vs: Vec<usize> = s.ones().collect();
        for i in 0..vs.len() {
            for j in (i + 1)..vs.len() {
                if !self.adj[vs[i]].contains(vs[j]) {
                    return false;
                }
            }
        }
        true
    }

    /// Is `s` an almost clique (some single vertex deletion makes it a clique)?
    fn is_almost_clique(&self, s: &Bits) -> bool {
        let vs: Vec<usize> = s.ones().collect();
        if vs.len() <= 2 {
            return true;
        }
        for skip in 0..vs.len() {
            let mut ok = true;
            'outer: for i in 0..vs.len() {
                if i == skip {
                    continue;
                }
                for j in (i + 1)..vs.len() {
                    if j == skip {
                        continue;
                    }
                    if !self.adj[vs[i]].contains(vs[j]) {
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

    /// Is `s` a minimal separator (at least two full components)?
    fn is_minimal_separator(&self, s: &Bits) -> bool {
        self.full_components(s).len() >= 2
    }

    /// Induced subgraph on `c ∪ s` with `s` completed into a clique (used for
    /// safe-separator decomposition).
    fn induced_with_clique(&self, c: &Bits, s: &Bits) -> BGraph {
        let mut verts = c.clone();
        verts.union_with(s);
        let idx: Vec<usize> = verts.ones().collect();
        let mut map = vec![usize::MAX; self.n];
        for (new, &old) in idx.iter().enumerate() {
            map[old] = new;
        }
        let m = idx.len();
        let mut adj = vec![Bits::with_capacity(m); m];
        for (new, &old) in idx.iter().enumerate() {
            for nb in self.adj[old].ones() {
                if verts.contains(nb) {
                    adj[new].insert(map[nb]);
                }
            }
        }
        let s_list: Vec<usize> = s.ones().collect();
        for i in 0..s_list.len() {
            for j in (i + 1)..s_list.len() {
                let (a, b) = (map[s_list[i]], map[s_list[j]]);
                adj[a].insert(b);
                adj[b].insert(a);
            }
        }
        BGraph { n: m, adj }
    }

    /// Find a safe separator, if one exists: a clique separator (always safe)
    /// or an almost-clique minimal separator (safe by Bodlaender–Koster).
    ///
    /// Candidates are the vertex neighbourhoods of the original graph and of a
    /// min-fill elimination (the separators of the resulting tree-decomposition).
    fn safe_separator(&self) -> Option<Bits> {
        let mut cands: Vec<Bits> = (0..self.n).map(|v| self.adj[v].clone()).collect();
        cands.extend(self.min_fill_neighborhoods());

        let mut seen: HashSet<Bits> = HashSet::new();
        for s in cands {
            if !seen.insert(s.clone()) {
                continue;
            }
            if s.count_ones(..) >= self.n {
                continue; // S = V(G) is not a separator
            }
            if self.components(&s).len() < 2 {
                continue; // not a separator
            }
            if self.is_clique(&s) {
                return Some(s);
            }
            if self.is_almost_clique(&s) && self.is_minimal_separator(&s) {
                return Some(s);
            }
        }
        None
    }

    /// Candidate separators from a deterministic min-fill elimination: the
    /// neighbourhood of each eliminated vertex (in the filled graph).
    fn min_fill_neighborhoods(&self) -> Vec<Bits> {
        let n = self.n;
        let mut adj = self.adj.clone();
        let mut alive = Bits::with_capacity(n);
        for v in 0..n {
            alive.insert(v);
        }
        let mut cands = Vec::new();

        loop {
            let mut best: Option<(usize, usize)> = None;
            for v in 0..n {
                if !alive.contains(v) {
                    continue;
                }
                let fill = fill_in_count(&adj, &alive, v);
                if best.map_or(true, |(_, bf)| fill < bf) {
                    best = Some((v, fill));
                }
            }
            let Some((v, _)) = best else { break; };

            let nv = neighbors_of(&adj, &alive, v);
            cands.push(nv.clone());

            let nv_list: Vec<usize> = nv.ones().collect();
            for i in 0..nv_list.len() {
                for j in (i + 1)..nv_list.len() {
                    let (a, b) = (nv_list[i], nv_list[j]);
                    adj[a].insert(b);
                    adj[b].insert(a);
                }
            }
            alive.set(v, false);
        }
        cands
    }
}

/// Neighbours of `v` among the alive vertices of a working graph.
fn neighbors_of(adj: &[Bits], alive: &Bits, v: usize) -> Bits {
    let mut r = adj[v].clone();
    r.intersect_with(alive);
    r
}

/// Fill-in count of eliminating `v` in a working graph.
fn fill_in_count(adj: &[Bits], alive: &Bits, v: usize) -> usize {
    let nv = neighbors_of(adj, alive, v);
    let list: Vec<usize> = nv.ones().collect();
    let d = list.len();
    if d < 2 {
        return 0;
    }
    let mut existing = 0usize;
    for i in 0..d {
        for j in (i + 1)..d {
            if adj[list[i]].contains(list[j]) {
                existing += 1;
            }
        }
    }
    d * (d - 1) / 2 - existing
}

const NO_CHILD: usize = usize::MAX;

/// A node of the block-sieve trie. Vertices are decided in order `0..n-1`; a
/// node with `vertex == n` is a leaf holding a stored set.
struct SieveNode {
    vertex: usize,
    include_child: usize,
    exclude_child: usize,
    stored: Option<Bits>,
}

/// A block sieve (§7.1): a trie storing O-block components `A` and supporting
/// superset queries `U`, returning every stored `A` with `U ⊆ A` and
/// `|N(U) ∪ N(A)| ≤ k + 1`, pruned by a margin bound on `k + 1 - |N(A)|`.
struct BlockSieve<'a> {
    g: &'a BGraph,
    k: usize,
    margin: usize,
    nodes: Vec<SieveNode>,
}

impl<'a> BlockSieve<'a> {
    fn new(g: &'a BGraph, k: usize, margin: usize) -> Self {
        BlockSieve {
            g,
            k,
            margin,
            nodes: vec![SieveNode {
                vertex: 0,
                include_child: NO_CHILD,
                exclude_child: NO_CHILD,
                stored: None,
            }],
        }
    }

    fn store(&mut self, a: &Bits) {
        let mut idx = 0usize;
        loop {
            let v = self.nodes[idx].vertex;
            if v >= self.g.n {
                self.nodes[idx].stored = Some(a.clone());
                return;
            }
            let take = a.contains(v);
            let child = if take {
                self.nodes[idx].include_child
            } else {
                self.nodes[idx].exclude_child
            };
            if child != NO_CHILD {
                idx = child;
            } else {
                let new_idx = self.nodes.len();
                self.nodes.push(SieveNode {
                    vertex: v + 1,
                    include_child: NO_CHILD,
                    exclude_child: NO_CHILD,
                    stored: None,
                });
                if take {
                    self.nodes[idx].include_child = new_idx;
                } else {
                    self.nodes[idx].exclude_child = new_idx;
                }
                idx = new_idx;
            }
        }
    }

    fn query(&self, u: &Bits, n_u: &Bits, out: &mut Vec<Bits>) {
        self.query_rec(0, u, n_u, out, 0);
    }

    fn query_rec(&self, idx: usize, u: &Bits, n_u: &Bits, out: &mut Vec<Bits>, i: usize) {
        // Prune: every stored set below has |N(U) ∪ N(A)| > k + 1 once
        // |N(U) ∩ prefix| exceeds the margin bound.
        if i > self.margin {
            return;
        }
        let node = &self.nodes[idx];
        let v = node.vertex;
        if v >= self.g.n {
            if let Some(a) = &node.stored {
                let n_a = self.g.neighborhood(a);
                let mut un = n_u.clone();
                un.union_with(&n_a);
                if un.count_ones(..) <= self.k + 1 {
                    out.push(a.clone());
                }
            }
            return;
        }
        if u.contains(v) {
            // v ∈ U, so v must be included; v ∉ N(U) since N(U) is open.
            if node.include_child != NO_CHILD {
                self.query_rec(node.include_child, u, n_u, out, i);
            }
        } else {
            if node.exclude_child != NO_CHILD {
                self.query_rec(node.exclude_child, u, n_u, out, i);
            }
            if node.include_child != NO_CHILD {
                let i2 = i + if n_u.contains(v) { 1 } else { 0 };
                self.query_rec(node.include_child, u, n_u, out, i2);
            }
        }
    }
}

/// Margin upper bounds `0 < m_1 < ... < m_t = k` for the block sieves.
fn margin_bounds(k: usize) -> Vec<usize> {
    let mut ms = Vec::new();
    let mut m = 2usize;
    while m < k {
        ms.push(m);
        m = m.saturating_mul(2);
    }
    ms.push(k);
    ms
}

/// Store an O-block component `a` into the sieve whose margin bound matches
/// `k + 1 - |N(a)|`.
fn store_o(g: &BGraph, sieves: &mut [BlockSieve], margins: &[usize], a: &Bits, k: usize) {
    let s = g.neighborhood(a).count_ones(..);
    let margin = k + 1 - s;
    let idx = margins
        .iter()
        .position(|&m| m >= margin)
        .unwrap_or(margins.len() - 1);
    sieves[idx].store(a);
}

/// Decide whether `tw(g) <= k` (g must be connected). Implements Algorithm
/// PID-BT of Tamaki.
fn decide(g: &BGraph, k: usize) -> bool {
    if g.n == 0 {
        return true;
    }

    // I = feasible I-block components (as a set, for membership).
    let mut i_set: HashSet<Bits> = HashSet::new();
    // O = feasible O-block components, stored in margin-bucketed block sieves.
    let margins = margin_bounds(k);
    let mut sieves: Vec<BlockSieve> = margins.iter().map(|&m| BlockSieve::new(g, k, m)).collect();
    let mut o_set: HashSet<Bits> = HashSet::new();
    // P = buildable potential maximal cliques.
    let mut p_list: Vec<Bits> = Vec::new();
    let mut p_set: HashSet<Bits> = HashSet::new();
    // S = feasible potential maximal cliques.
    let mut s_list: Vec<Bits> = Vec::new();
    let mut s_set: HashSet<Bits> = HashSet::new();
    // C_1..C_j = generated I-block components (to be processed in order).
    let mut c_list: Vec<Bits> = Vec::new();
    let mut c_set: HashSet<Bits> = HashSet::new();

    // Step 4: initialize from closed neighbourhoods N[v].
    for v in 0..g.n {
        let nv = g.closed_neighborhood(v);
        if nv.count_ones(..) > k + 1 || !g.is_pmc(&nv) {
            continue;
        }
        if p_set.insert(nv.clone()) {
            p_list.push(nv.clone());
        }
        if g.support(&nv).is_empty() {
            if s_set.insert(nv.clone()) {
                s_list.push(nv.clone());
            }
            let out = g.outlet(&nv);
            if !out.is_clear() {
                let c = g.crib(&out, &nv);
                if c_set.insert(c.clone()) {
                    c_list.push(c);
                }
            }
        }
    }

    // Steps 5-6: process I-blocks in order, generating new ones.
    let mut i = 0usize;
    while i < c_list.len() {
        let ci = c_list[i].clone();
        i += 1;
        i_set.insert(ci.clone());

        let nci = g.neighborhood(&ci);

        // Step 6(a)iii: combine C_i with each existing O-block. The block
        // sieves return exactly the O-blocks B with C_i ⊆ B and
        // |N(C_i) ∪ N(B)| ≤ k + 1.
        let mut candidates: Vec<Bits> = Vec::new();
        for s in &sieves {
            s.query(&ci, &nci, &mut candidates);
        }
        let mut new_o: Vec<Bits> = Vec::new();

        for b in &candidates {
            let mut kset = nci.clone();
            kset.union_with(&g.neighborhood(b));
            if kset.count_ones(..) > k + 1 {
                continue;
            }
            if g.is_pmc(&kset) {
                if p_set.insert(kset.clone()) {
                    p_list.push(kset.clone());
                }
            }
            if kset.count_ones(..) <= k {
                if let Some(a) = g.outbound_full_component(&kset) {
                    if o_set.insert(a.clone()) {
                        store_o(g, &mut sieves, &margins, &a, k);
                        new_o.push(a);
                    }
                }
            }
        }

        // Step 6(a)iv: the outbound full component of N(C_i).
        if let Some(a) = g.outbound_full_component(&nci) {
            if o_set.insert(a.clone()) {
                store_o(g, &mut sieves, &margins, &a, k);
                new_o.push(a);
            }
        }

        // Step 6(a)v: build PMCs from the newly-added O-blocks.
        for a in &new_o {
            let na = g.neighborhood(a);
            for v in na.ones() {
                let mut kset = na.clone();
                let mut nv_cap_a = g.adj[v].clone();
                nv_cap_a.intersect_with(a);
                kset.union_with(&nv_cap_a);
                if kset.count_ones(..) <= k + 1 && g.is_pmc(&kset) {
                    if p_set.insert(kset.clone()) {
                        p_list.push(kset.clone());
                    }
                }
            }
        }

        // Step 6(a)vi: promote buildable PMCs whose support is feasible.
        for kset in &p_list {
            if s_set.contains(kset) {
                continue;
            }
            let sup = g.support(kset);
            if sup.iter().all(|c| i_set.contains(c)) {
                s_set.insert(kset.clone());
                s_list.push(kset.clone());
                let out = g.outlet(kset);
                if !out.is_clear() {
                    let c = g.crib(&out, kset);
                    if c_set.insert(c.clone()) {
                        c_list.push(c);
                    }
                }
            }
        }
    }

    // Step 7.
    s_list.iter().any(|k| g.outlet(k).is_clear())
}

/// Treewidth of a connected graph, with safe-separator decomposition.
fn tw_connected(g: &BGraph) -> usize {
    if g.n <= 1 {
        return 0;
    }

    // Decompose across a safe separator if one is found: completing S into a
    // clique does not change tw, and tw(G) = max over components C of G-S of
    // tw(G[C ∪ S] + clique(S)).
    if let Some(s) = g.safe_separator() {
        let mut tw = 0usize;
        for c in g.components(&s) {
            let sub = g.induced_with_clique(&c, &s);
            tw = tw.max(tw_connected(&sub));
        }
        return tw;
    }

    let mut k = g.min_degree();
    loop {
        if decide(g, k) {
            return k;
        }
        k += 1;
        if k >= g.n {
            return g.n.saturating_sub(1);
        }
    }
}

/// Compute the treewidth of `g` using Tamaki's PID-BT algorithm.
pub fn pidd_tw(g: &crate::graph::Graph) -> usize {
    let bg = BGraph::from_crate(g);
    if bg.n <= 1 {
        return 0;
    }
    let comps = bg.components(&Bits::with_capacity(bg.n));
    let mut tw = 0usize;
    for c in comps {
        let sub = bg.induced_subgraph(&c);
        tw = tw.max(tw_connected(&sub));
    }
    tw
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::bb;
    use crate::graph::Graph;

    fn grid3x3() -> Graph {
        let mut edges = Vec::new();
        for r in 0..3 {
            for c in 0..3 {
                let i = r * 3 + c;
                if c + 1 < 3 {
                    edges.push((i, i + 1));
                }
                if r + 1 < 3 {
                    edges.push((i, i + 3));
                }
            }
        }
        Graph::from_edges(edges)
    }

    fn all_graphs(n: usize) -> Vec<Graph> {
        let max = n.saturating_mul(n.saturating_sub(1)) / 2;
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in (i + 1)..n {
                pairs.push((i, j));
            }
        }
        let mut out = Vec::with_capacity(1usize << max);
        for mask in 0u64..(1u64 << max) {
            let mut g = Graph::with_capacity(n);
            for (k, &(a, b)) in pairs.iter().enumerate() {
                if (mask >> k) & 1 == 1 {
                    g.add_edge(NodeIndex::new(a), NodeIndex::new(b));
                }
            }
            out.push(g);
        }
        out
    }

    #[test]
    fn known_values() {
        assert_eq!(pidd_tw(&Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4)])), 1); // P5
        assert_eq!(pidd_tw(&Graph::from_edges([(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)])), 2); // C5
        assert_eq!(pidd_tw(&Graph::from_edges([(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)])), 3); // K4
        assert_eq!(pidd_tw(&grid3x3()), 3);
        assert_eq!(pidd_tw(&Graph::with_capacity(0)), 0);
        assert_eq!(pidd_tw(&Graph::with_capacity(1)), 0);
    }

    #[test]
    fn agrees_with_quickbb_up_to_6_vertices() {
        for n in 0..=6usize {
            for g in all_graphs(n) {
                let expected = bb(&g).treewidth;
                let got = pidd_tw(&g);
                assert_eq!(got, expected, "n={n}: pidd tw {got} != quickbb tw {expected}");
            }
        }
    }

    #[test]
    fn agrees_with_quickbb_on_random_graphs() {
        use crate::generator::random::{gnm, gnp};
        use rand::Rng;
        use rand::SeedableRng;
        use rand::rngs::StdRng;

        let mut rng = StdRng::seed_from_u64(0xDEC0DE);
        for n in 7..=11usize {
            for _ in 0..30 {
                let g = gnp(n, 0.5, &mut rng);
                assert_eq!(pidd_tw(&g), bb(&g).treewidth, "gnp n={n}");
            }
        }
        let mut rng = StdRng::seed_from_u64(0xBEEF);
        for n in 7..=11usize {
            let max_edges = n * (n - 1) / 2;
            for _ in 0..30 {
                let m = rng.random_range(n..=max_edges);
                let g = gnm(n, m, &mut rng);
                assert_eq!(pidd_tw(&g), bb(&g).treewidth, "gnm n={n} m={m}");
            }
        }
    }

    #[test]
    fn agrees_with_quickbb_on_named_graphs() {
        use crate::generator::named::{cycle, grid, path};
        for n in 2..=10 {
            let g = path(n);
            assert_eq!(pidd_tw(&g), bb(&g).treewidth, "path({n})");
        }
        for n in 3..=9 {
            let g = cycle(n);
            assert_eq!(pidd_tw(&g), bb(&g).treewidth, "cycle({n})");
        }
        for (r, c) in [(3, 3), (3, 4), (4, 3), (4, 4), (5, 5)] {
            let g = grid(r, c);
            assert_eq!(pidd_tw(&g), bb(&g).treewidth, "grid({r}x{c})");
        }
    }
}
