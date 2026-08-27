use cartographer::io::export_graph3;
use cartographer::{Graph, bb_tw, generator::small, pidd_tw};

fn main() {
    let peterson = small::petersen();
    export_graph3(&peterson, "graphs/petersen.graph3").expect("Failed to export graph");
    // wagner cubical octahedron bowtie gem
    let wagner = small::wagner();
    export_graph3(&wagner, "graphs/wagner.graph3").expect("Failed to export graph");

    let wagner = small::cubical();
    export_graph3(&wagner, "graphs/cubical.graph3").expect("Failed to export graph");
    let wagner = small::octahedron();
    export_graph3(&wagner, "graphs/octahedron.graph3").expect("Failed to export graph");
    let wagner = small::bowtie();
    export_graph3(&wagner, "graphs/bowtie.graph3").expect("Failed to export graph");
    let wagner = small::gem();
    export_graph3(&wagner, "graphs/gem.graph3").expect("Failed to export graph");
}
