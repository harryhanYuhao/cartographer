use crate::io::{append_to_file, export_graph3};
use crate::{
    Graph,
    algorithm::{bb_tw, pidd_tw},
    generator::{gnp_n, zx::rand_graph_like_zx},
    operation::reduce_had_triangle_total,
};
use rand::SeedableRng;
use rand::rngs::StdRng;

fn reduce_had_total() {
    let n = 1000;
    let mut count = 0;
    let mut tot_tw1 = 0;
    let mut tot_tw2 = 0;
    for i in 0..n {
        let mut rng = StdRng::seed_from_u64((i % 7) * 11319 + i * 97);
        let g = rand_graph_like_zx(10, 28, &mut rng);
        let filename = "graphs/zx-dot.graph3";
        export_graph3(&g, &filename).expect("Failed to export graph");
        // append_to_file(&format!("# Treewidht: {}", pidd_tw(&g)), &filename).unwrap();
        // let g = Graph::from_graph3_file("./graphs/graph3_try.graph3").unwrap();

        let tw1 = pidd_tw(&g);
        let g = reduce_had_triangle_total(&g);
        export_graph3(&g, "graphs/zx-dot-red.graph3").expect("Failed to export graph");
        let tw2 = pidd_tw(&g);
        println!("{}: {} -> {}", i, tw1, tw2);

        if tw2 != tw1 {
            count += 1;
        }

        tot_tw1 += tw1;
        tot_tw2 += tw2;
    }

    println!(
        "Count of graphs with different treewidth after reduction: {} out of {}",
        count, n
    );

    println!(
        "Average treewidth before reduction: {:.2}, after reduction: {:.2}",
        tot_tw1 as f64 / n as f64,
        tot_tw2 as f64 / n as f64
    );
}
