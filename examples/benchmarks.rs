use cartographer::io::{append_to_file, create_or_replace_file};
use cartographer::{
    algorithm::pidd_tw, generator::zx::rand_graph_like_zx, operation::reduce_had_triangle_total,
};
use rand::prelude::*;
use std::thread;
use std::time::Duration;

const CSV_HEADER: &str = "nodes, edges, tw_before, tw_after\n";

/// Cooldown between graphs, in milliseconds. Long runs (hours) otherwise pin
/// every core at 100%; set to 0 to disable.
const REST_MS: u64 = 5;

/// Benchmark one (n, e) cell: `iterations` random graphs, returning the CSV
/// rows. Owns its RNG — each thread must seed its own stream.
fn benchmark(n: usize, e: usize, iterations: usize, seed: u64) -> String {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut ret = String::new();
    for _ in 0..iterations {
        let g = rand_graph_like_zx(n, e, &mut rng);
        let tw_b = pidd_tw(&g);

        let g = reduce_had_triangle_total(&g);
        let tw_a = pidd_tw(&g);

        ret += &format!("{}, {}, {}, {}\n", n, e, tw_b, tw_a);

        // Cool down after each intense burst (one graph) so a multi-hour
        // run leaves the machine responsive.
        thread::sleep(Duration::from_millis(REST_MS));
    }
    ret
}

/// One dispatcher per vertex count: sample `outer_iter` edge counts, run
/// every sample on its own thread, then append the rows in sample order.
fn dispatcher(n: usize, seed: u64, inner_iter: usize, outer_iter: usize) {
    let mut rng = StdRng::seed_from_u64(seed);

    let filename = format!("tw_{}_vertex.csv", n);

    println!("[n={n}] starting: {outer_iter} samples x {inner_iter} graphs each");

    create_or_replace_file(CSV_HEADER, &filename).unwrap();
    let total_edges = n * (n - 1) / 2;
    let mut lower_bound = total_edges / 10;
    if lower_bound < 1 {
        lower_bound = 1;
    }
    let mut upper_bound = (total_edges as f64 * 0.9) as usize;

    if n > 20 {
        upper_bound /= 6;
    } else if n > 15 {
        upper_bound /= 2;
    }

    let numbers: Vec<i32> = (lower_bound as i32..upper_bound as i32).collect();

    let random_vec: Vec<i32> = (0..outer_iter)
        .map(|_| *numbers.choose(&mut rng).unwrap())
        .collect();

    // One thread per sampled edge count. Each gets its own seed so repeated
    // samples of the same e still produce different graphs. The edge count
    // is copied out of the borrow before the move closure: spawned threads
    // must own all their data.
    let threads: Vec<thread::JoinHandle<String>> = random_vec
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let seed_i = seed.wrapping_add(i as u64);
            let e = *e as usize;
            thread::spawn(move || benchmark(n, e, inner_iter, seed_i))
        })
        .collect();

    // Join in sample order, so the CSV rows are deterministic.
    let mut done = 0;
    for t in threads {
        let rows = t.join().unwrap();
        let count = rows.lines().count();
        append_to_file(&rows, &filename).unwrap();
        done += 1;
        println!("[n={n}] {done}/{outer_iter} samples appended ({count} rows)");
    }
    println!("[n={n}] wrote {} -> {filename}", outer_iter * inner_iter);
}

fn main() {
    let seed = 194131;
    let inner_iter = 3;
    let mut outer_iter = 35;

    // One dispatcher thread per vertex count.
    let mut threads = vec![];

    for i in 5..30 {
        if i % 4 == 0 {
            outer_iter -= 5
        }
        threads.push(thread::spawn(move || {
            dispatcher(i, seed, inner_iter, outer_iter);
        }));
    }
    for t in threads {
        t.join().unwrap();
    }
    println!("all dispatchers finished");
}
