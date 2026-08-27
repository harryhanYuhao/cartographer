//! Graph-like ZX-diagram generators.
//!
//! A *graph-like* ZX-diagram (Backens, "Making the stabilizer ZX-calculus
//! complete for computing pure qubit stabilizer amplitudes") is a ZX-diagram
//! whose nodes are all Z spiders and whose edges are all Hadamard (H) edges.
//! Such diagrams are in one-to-one correspondence with graphs: the spiders
//! are the vertices and the H edges are the edges.
//!
//! [`rand_graph_like_zx`] samples the *topology* from the Erdős–Rényi
//! *G(n, e)* model (see [`crate::generator::gnm`]) and then labels every
//! vertex [`VColor::Z`] and every edge [`EColor::H`].

use crate::generator::gnm;
use crate::graph::{EColor, Graph, VColor};
use rand::Rng;

/// Generate a random graph-like ZX-diagram with `n` Z-spiders and `e`
/// H-edges.
///
/// The topology is `G(n, e)` (exactly `e` distinct edges sampled uniformly,
/// clamped to `[0, C(n, 2)]`). Every vertex is a Z spider with phase 0 and
/// every edge is a Hadamard edge.
pub fn rand_graph_like_zx(n: usize, e: usize, rng: &mut impl Rng) -> Graph {
    let mut g = gnm(n, e, rng);

    // All nodes are Z spiders.
    let vertices: Vec<_> = g.alive_vertices().collect();
    for v in vertices {
        g.set_color(v, VColor::Z(0));
    }

    // All edges are H (Hadamard) edges.
    let edges: Vec<_> = g.edges().map(|(_, _, ed)| ed).collect();
    for ed in edges {
        g.set_edge_color(ed, EColor::H);
    }
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::EColor;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn seeded() -> StdRng {
        StdRng::seed_from_u64(0x5EED)
    }

    #[test]
    fn all_nodes_z_and_all_edges_h() {
        let mut rng = seeded();
        let g = rand_graph_like_zx(12, 20, &mut rng);
        assert_eq!(g.node_count(), 12);
        assert_eq!(g.edge_count(), 20);
        for v in g.alive_vertices() {
            assert_eq!(g.label(v), VColor::Z(0), "vertex {v:?} not a Z spider");
        }
        let mut seen = 0;
        for (_, _, ed) in g.edges() {
            assert_eq!(g.edge_color(ed), EColor::H, "edge {ed:?} not an H edge");
            seen += 1;
        }
        assert_eq!(seen, 20);
    }

    #[test]
    fn edge_less_zx_diagram() {
        let mut rng = seeded();
        let g = rand_graph_like_zx(5, 0, &mut rng);
        assert_eq!(g.node_count(), 5);
        assert_eq!(g.edge_count(), 0);
        for v in g.alive_vertices() {
            assert_eq!(g.label(v), VColor::Z(0));
        }
    }

    #[test]
    fn edge_count_clamped_like_gnm() {
        let mut rng = seeded();
        // Requesting more edges than C(6, 2) = 15 saturates at 15.
        let g = rand_graph_like_zx(6, 100, &mut rng);
        assert_eq!(g.node_count(), 6);
        assert_eq!(g.edge_count(), 15);
        for (_, _, ed) in g.edges() {
            assert_eq!(g.edge_color(ed), EColor::H);
        }
    }
}
