use cartographer::io::{append_to_file, export_graph3};
use cartographer::{
    Graph,
    algorithm::{bb_tw, pidd_tw},
    generator::{gnp_n, zx::rand_graph_like_zx},
    operation::{k3_remove, k3_remove_had2},
};

use rand::SeedableRng;
use rand::rngs::StdRng;

fn main() {
    let mut rng = StdRng::seed_from_u64(10);
    let g = rand_graph_like_zx(10, 15, &mut rng);
    let filename = "graphs/zx-dot.graph3";
    // export_graph3(&g, &filename).expect("Failed to export graph");
    // append_to_file(&format!("# Treewidht: {}", pidd_tw(&g)), &filename).unwrap();
    // let g = Graph::from_graph3_file("./graphs/graph3_try.graph3").unwrap();

    println!("Treewidth: {}", pidd_tw(&g));

    let g = k3_remove_had2(&g);
    export_graph3(&g, "graphs/zx-dot-red.graph3").expect("Failed to export graph");
    println!("Treewidth: {}", pidd_tw(&g));
}
