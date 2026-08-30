use cartographer::io::{append_to_file, create_or_replace_file};
use cartographer::{
    algorithm::pidd_tw,
    generator::zx::rand_graph_like_zx,
    operation::{line_graph, reduce_had_triangle_total},
};
use rand::prelude::*;
use std::fs;
use std::thread;
use std::time::{Duration, SystemTime};

const CSV_HEADER: &str = "nodes before, nodes after, tw_before, tw_after\n";

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

        let line_g = line_graph(&g);
        let tw_b = pidd_tw(&line_g);
        let node_before = line_g.node_count();

        let g = reduce_had_triangle_total(&g);

        let line_g = line_graph(&g);
        let tw_a = pidd_tw(&line_g);
        let node_after = line_g.node_count();

        ret += &format!("{}, {}, {}, {}\n", node_before, node_after, tw_b, tw_a);

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

    let filename = format!("tw_line_g/tw_{}_vertex_line.csv", n);

    println!("[n={n}] starting: {outer_iter} samples x {inner_iter} graphs each");

    match fs::metadata(&filename) {
        Ok(_) => {}
        Err(_) => {
            create_or_replace_file(CSV_HEADER, &filename).unwrap();
        }
    }

    let total_edges = n * (n - 1) / 2;
    let mut lower_bound = total_edges / 10;
    if lower_bound < 1 {
        lower_bound = 1;
    }
    let upper_bound = (total_edges as f64 * 0.95) as usize;

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
    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let seed = time.as_secs() as u64;
    let inner_iter = 1;
    let mut outer_iter = 10;

    // One dispatcher thread per vertex count.
    let mut threads = vec![];

    for i in 10..15 {
        if i % 4 == 0 {
            outer_iter -= 4
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
