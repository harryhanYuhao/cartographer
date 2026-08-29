use cartographer::io::{append_to_file, export_graph3};
use cartographer::{
    Graph,
    algorithm::{bb_tw, pidd_tw},
    generator::{gnp_n, zx::rand_graph_like_zx},
    operation::reduce_had_triangle_total,
    q_circuit::QCircuit,
};

fn benchmark(n: usize, e: usize, seed: u64) {}
fn main() {}
