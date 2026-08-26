use cartographer::{Graph, bb_tw, generator::random, pidd_tw};

fn main() {
    let num = 35;
    let total = (num * (num - 1)) as f64 / 2.0;

    let g = random::gnp_n(num, 0.6);

    println!("{}", pidd_tw(&g));
    println!("{}", bb_tw(&g));
}
