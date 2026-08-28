//! Cartographer: a small graph-theory library with an exact treewidth solver.
//!
//! The headline algorithm is **QuickBB** — a complete branch-and-bound
//! algorithm for the treewidth of an undirected graph, as described in
//! *A Complete Anytime Algorithm for Treewidth* (Gogate & Dechter, UAI 2004).
//! It combines the base branch-and-bound (Figure 4) with the minor-min-width
//! lower bound (Figure 1), a single-pass deterministic min-fill upper bound,
//! and the enhancements of §5–6: simplicial/almost-simplicial graph
//! reduction, neighbour-only branching, safe edge addition, and fill-in
//! dominance pruning.
//!
//! The graph representation is built on [`petgraph::graph::UnGraph`] with
//! logical (masked) vertex deletion, so `NodeIndex` values stay stable across
//! elimination operations.
//!
//! # Layout
//!
//! - [`graph`] — the core [`Graph`] data structure.
//! - [`algorithm`] — two exact treewidth solvers: QuickBB branch-and-bound
//!   ([`bb`], [`bb_tw`]) and Tamaki's PID-BT dynamic programming
//!   ([`pidd::pidd_tw`][algorithm::pidd::pidd_tw]), plus their bounds
//!   ([`min_fill`][algorithm::min_fill], [`minor_min_width`][algorithm::mmw]).
//! - [`generator`] — named graphs ([`path`][generator::named::path],
//!   [`cycle`][generator::named::cycle], [`clique`][generator::named::clique],
//!   [`petersen`][generator::small::petersen], [`wheel`][generator::named::wheel],
//!   [`hypercube`][generator::named::hypercube], …) and random graph models
//!   ([`gnp`][generator::random::gnp], [`gnm`][generator::random::gnm],
//!   [`random_regular`][generator::random::random_regular],
//!   [`barabasi_albert`][generator::random::barabasi_albert],
//!   [`random_graph`][generator::random::random_graph]).
//!
//! For convenience, [`Graph`], [`VColor`], and [`EColor`] are re-exported at
//! the crate root.
//!
//! # Example
//! ```
//! use cartographer::graph::Graph;
//! use cartographer::algorithm::bb;
//!
//! // Path 0-1-2-3-4, treewidth 1.
//! let g = Graph::from_edges([(0,1),(1,2),(2,3),(3,4)]);
//! let r = bb(&g);
//! assert_eq!(r.treewidth, 1);
//! assert_eq!(r.order.len(), 5);
//! ```

pub mod algorithm;
pub mod generator;
pub mod graph;
pub mod io;
pub mod operation;

mod cli;

pub use graph::{EColor, Graph, VColor};
