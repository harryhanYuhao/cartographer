//! Random graph generators.
//!
//! All generators take a `&mut impl Rng` so callers can supply a seeded RNG
//! (e.g. [`rand::rngs::StdRng::seed_from_u64`]) for reproducible output, and
//! return a fresh [`Graph`] with vertices indexed `0..n`.
//!
//! Models implemented:
//!
//! - [`gnp`]: Erdős–Rényi *G(n, p)* — each edge present independently with
//!   probability `p`.
//! - [`gnm`]: Erdős–Rényi *G(n, m)* — exactly `m` distinct edges chosen
//!   uniformly at random.
//! - [`random_regular`]: a simple graph in which every vertex has degree
//!   exactly `k` (configuration model with rejection sampling).
//! - [`barabasi_albert`]: scale-free graph grown by preferential attachment.
//!
//! [`random_graph`] is a convenience dispatcher that picks one of these
//! models (and a reasonable parameter) at random.

use crate::graph::Graph;
use petgraph::graph::NodeIndex;
use rand::Rng;
use rand::seq::SliceRandom;

/// Make `n` isolated vertices in a fresh graph. Used internally so that
/// vertex indices are always `0..n` regardless of the edges we add later.
fn with_n_vertices(n: usize) -> Graph {
    Graph::with_capacity(n)
}

/// **Erdős–Rényi G(n, p).** Construct a graph on `n` vertices in which each
/// unordered pair `{u, v}` with `u < v` is present independently with
/// probability `p`.
///
/// `p` is clamped to `[0, 1]`. Expected edge count is `p * n * (n-1) / 2`.
pub fn gnp(n: usize, p: f64, rng: &mut impl Rng) -> Graph {
    let p = p.clamp(0.0, 1.0);
    let mut g = with_n_vertices(n);
    for u in 0..n {
        for v in (u + 1)..n {
            if rng.random::<f64>() < p {
                g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
            }
        }
    }
    g
}

pub fn gnp_n(n: usize, p: f64) -> Graph {
    gnp(n, p, &mut rand::rng())
}

/// **Erdős–Rényi G(n, m).** Construct a graph on `n` vertices with exactly
/// `m` distinct edges chosen uniformly at random from all `C(n, 2)` possible
/// edges.
///
/// `m` is clamped to `[0, C(n, 2)]`. For dense requests the complement is
/// sampled instead so the work stays `O(C(n,2))`.
pub fn gnm(n: usize, m: usize, rng: &mut impl Rng) -> Graph {
    let max_edges = n.saturating_mul(n.saturating_sub(1)) / 2;
    let m = m.min(max_edges);

    let mut g = with_n_vertices(n);
    if m == 0 || n < 2 {
        return g;
    }

    // Sample `m` distinct edge slots without replacement. For dense requests
    // (more than half the possible edges), sample the complement instead.
    let mut slots: Vec<usize> = (0..max_edges).collect();
    slots.shuffle(rng);

    // Map a slot index k to an edge (u, v) with u < v.
    let choose = |k: usize| -> (usize, usize) {
        // Find the largest u such that u*(2n - u - 1)/2 <= k.
        let mut lo = 0usize;
        let mut hi = n - 1;
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            // Number of edges starting at an index < mid.
            let before = mid * (2 * n - mid - 1) / 2;
            if before <= k {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        let u = lo;
        let before = u * (2 * n - u - 1) / 2;
        let v = u + 1 + (k - before);
        (u, v)
    };

    for &k in slots.iter().take(m) {
        let (u, v) = choose(k);
        g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
    }
    g
}

/// **Random k-regular graph.** Construct a simple undirected graph on `n`
/// vertices in which every vertex has degree exactly `k`, via the
/// configuration (pairing) model with rejection of multi-edge/self-loop
/// pairings.
///
/// Requires `n * k` even and `k < n`; otherwise an empty graph is returned
/// (these are the standard feasibility conditions for a simple k-regular
/// graph). Up to a bounded number of retry attempts are made before giving
/// up and returning an empty graph, so inputs near the feasibility boundary
/// may occasionally yield an empty result.
pub fn random_regular(n: usize, k: usize, rng: &mut impl Rng) -> Graph {
    if n == 0 || k == 0 {
        return with_n_vertices(n);
    }
    if k >= n || !(n * k).is_multiple_of(2) {
        return with_n_vertices(n);
    }

    // Try the pairing model a bounded number of times; reject any sample
    // that produces a self-loop or a duplicate edge.
    for _ in 0..64 {
        let mut stubs: Vec<usize> = (0..n).flat_map(|v| std::iter::repeat_n(v, k)).collect();
        stubs.shuffle(rng);

        let mut g = with_n_vertices(n);
        let mut ok = true;
        let mut i = 0;
        while i + 1 < stubs.len() {
            let a = stubs[i];
            let b = stubs[i + 1];
            i += 2;
            if a == b || g.has_edge(NodeIndex::new(a), NodeIndex::new(b)) {
                ok = false;
                break;
            }
            g.add_edge(NodeIndex::new(a), NodeIndex::new(b));
        }
        if ok {
            return g;
        }
    }

    // Could not realize the degree sequence in the allotted attempts.
    with_n_vertices(n)
}

/// **Barabási–Albert preferential-attachment graph.** Grow a graph on `n`
/// vertices: start from a clique of `m + 1` vertices, then add each new
/// vertex one at a time, connecting it to `m` distinct existing vertices
/// chosen with probability proportional to their current degree.
///
/// Requires `n >= 1` and `m >= 1`. For `n <= m + 1` the seed clique is
/// returned. `m` is clamped to be at least 1.
pub fn barabasi_albert(n: usize, m: usize, rng: &mut impl Rng) -> Graph {
    let m = m.max(1);
    let mut g = with_n_vertices(n);
    if n == 0 {
        return g;
    }

    let seed = m + 1;
    // Seed: a clique on the first `min(seed, n)` vertices.
    let seed_size = seed.min(n);
    for u in 0..seed_size {
        for v in (u + 1)..seed_size {
            g.add_edge(NodeIndex::new(u), NodeIndex::new(v));
        }
    }

    // Grow the graph one vertex at a time.
    for new_v in seed_size..n {
        // Build a list in which each vertex appears once per incident endpoint
        // (twice per edge); sampling from it realizes preferential attachment.
        let mut pool: Vec<usize> = Vec::new();
        for v in 0..new_v {
            let d = g.degree(NodeIndex::new(v));
            for _ in 0..d {
                pool.push(v);
            }
        }

        let mut picked: Vec<usize> = Vec::with_capacity(m.min(new_v));
        while picked.len() < m.min(new_v) {
            if pool.is_empty() {
                break;
            }
            let idx = rng.random_range(0..pool.len());
            let cand = pool[idx];
            // Remove all occurrences of cand from the pool so we don't pick it twice.
            pool.retain(|&x| x != cand);
            picked.push(cand);
        }

        for &u in &picked {
            g.add_edge(NodeIndex::new(u), NodeIndex::new(new_v));
        }
    }

    g
}

/// **Top-level random-graph dispatcher.** Pick one of the four models above
/// at (roughly) equal probability and choose a parameter sized to `n` that
/// keeps the result neither empty nor a near-clique.
///
/// Guarantees `node_count == n` (for `n >= 0`). The specific model and
/// parameter are chosen by `rng`.
pub fn random_graph(n: usize) -> Graph {
    let mut rng_t = rand::rng();
    let mut rng = &mut rng_t;
    if n < 2 {
        return with_n_vertices(n);
    }

    let model: u8 = rng.random_range(0..4);
    match model {
        0 => {
            // G(n, p): p in [0.1, 0.5] — sparse but connected-with-high-probability.
            let p = 0.1 + rng.random::<f64>() * 0.4;
            gnp(n, p, rng)
        }
        1 => {
            // G(n, m): m between ~10% and ~50% of all possible edges.
            let max_edges = n * (n - 1) / 2;
            let lo = max_edges / 10;
            let hi = max_edges / 2 + 1;
            let m = if hi > lo {
                rng.random_range(lo..hi)
            } else {
                lo
            };
            gnm(n, m, rng)
        }
        2 => {
            // Random regular: k in {2, 3, 4}, constrained to keep n*k even.
            let mut k = rng.random_range(2..=4).min(n - 1);
            if !(n * k).is_multiple_of(2) {
                k = k.saturating_sub(1).max(2);
            }
            random_regular(n, k, rng)
        }
        _ => {
            // Barabási–Albert: m in {2, 3}.
            let m = rng.random_range(2..=3);
            barabasi_albert(n, m, rng)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithm::minor_min_width;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn seeded() -> StdRng {
        StdRng::seed_from_u64(0xC0FFEE)
    }

    #[test]
    fn gnp_vertex_count_and_density() {
        let mut rng = seeded();
        let g = gnp(20, 0.5, &mut rng);
        assert_eq!(g.node_count(), 20);
        // Expected ~95 edges; check it is in a sane band (no exact check: it's random).
        let edges = edge_count(&g);
        assert!(edges > 0 && edges < 20 * 19 / 2);
    }

    #[test]
    fn gnp_p_zero_is_empty_and_p_one_is_clique() {
        let mut rng = seeded();
        let empty = gnp(6, 0.0, &mut rng);
        assert_eq!(edge_count(&empty), 0);
        let full = gnp(6, 1.0, &mut rng);
        assert_eq!(edge_count(&full), 6 * 5 / 2);
    }

    #[test]
    fn gnm_exact_edge_count() {
        let mut rng = seeded();
        let g = gnm(10, 7, &mut rng);
        assert_eq!(g.node_count(), 10);
        assert_eq!(edge_count(&g), 7);
    }

    #[test]
    fn gnm_clamps_to_max_edges() {
        let mut rng = seeded();
        // Request far more edges than possible: should saturate at C(6,2)=15.
        let g = gnm(6, 1000, &mut rng);
        assert_eq!(edge_count(&g), 15);
    }

    #[test]
    fn random_regular_is_k_regular() {
        let mut rng = seeded();
        let n = 10;
        let k = 3;
        let g = random_regular(n, k, &mut rng);
        assert_eq!(g.node_count(), n);
        for v in 0..n {
            assert_eq!(g.degree(NodeIndex::new(v)), k, "vertex {v} not {k}-regular");
        }
    }

    #[test]
    fn random_regular_infeasible_returns_empty() {
        let mut rng = seeded();
        // n*k odd: no simple 3-regular graph on 5 vertices.
        let g = random_regular(5, 3, &mut rng);
        assert_eq!(edge_count(&g), 0);
        // k >= n: infeasible.
        let g = random_regular(4, 4, &mut rng);
        assert_eq!(edge_count(&g), 0);
    }

    #[test]
    fn barabasi_albert_vertex_count_and_connected() {
        let mut rng = seeded();
        let n = 30;
        let g = barabasi_albert(n, 2, &mut rng);
        assert_eq!(g.node_count(), n);
        // BA graphs are connected (seed is a clique, every new vertex adds >=1 edge).
        // Minimum degree of an attached vertex is at least 1 except the seed clique,
        // and the seed clique is itself connected.
        assert!(is_connected(&g, n), "BA graph should be connected");
    }

    #[test]
    fn barabasi_albert_seed_only_when_small() {
        let mut rng = seeded();
        // n == m+1 == 3: just the seed clique K3.
        let g = barabasi_albert(3, 2, &mut rng);
        assert_eq!(g.node_count(), 3);
        assert_eq!(edge_count(&g), 3);
    }

    #[test]
    fn random_graph_vertex_count_and_lb() {
        for n in [2usize, 5, 10, 20] {
            let g = random_graph(n);
            assert_eq!(g.node_count(), n, "wrong node count for n={n}");
            // Treewidth lower bound must be a sane non-negative value.
            let lb = minor_min_width(&g);
            assert!(lb < n, "MMW lb {lb} should be < n={n}");
        }
    }

    // ---- helpers ------------------------------------------------------------

    fn edge_count(g: &Graph) -> usize {
        // Each undirected edge is counted once.
        let mut total = 0usize;
        for v in 0..g.node_count() {
            total += g.degree(NodeIndex::new(v));
        }
        total / 2
    }

    fn is_connected(g: &Graph, n: usize) -> bool {
        if n == 0 {
            return true;
        }
        let mut seen = vec![false; n];
        let mut stack = vec![0usize];
        seen[0] = true;
        let mut count = 1;
        while let Some(u) = stack.pop() {
            for nb in g.alive_neighbors(NodeIndex::new(u)) {
                let j = nb.index();
                if !seen[j] {
                    seen[j] = true;
                    count += 1;
                    stack.push(j);
                }
            }
        }
        count == n
    }
}
