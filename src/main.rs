use cartographer::io::{append_to_file, export_graph3};
use cartographer::{
    Graph,
    algorithm::{bb_tw, pidd_tw},
    generator::{gnp_n, zx::rand_graph_like_zx},
    operation::reduce_had_triangle_total,
    q_circuit::QCircuit,
};

use rand::{Rng, SeedableRng, rngs::StdRng};

fn main() {
    let mut rng = StdRng::seed_from_u64(412);
    let q = QCircuit::rand_circuit(3, 12, &mut rng);

    let g = q.to_graph();
    // let g = reduce_had_triangle_total(&g);

    println!("TW:{}", pidd_tw(&g));

    let filename = "graphs/qcircuit.graph3";
    export_graph3(&g, &filename).expect("Failed to export graph");
}
