//! Cartographer: a small graph-theory library with an exact treewidth solver.
//!
//! The headline algorithm is **QuickBB** — a branch-and-bound algorithm for
//! the treewidth of an undirected graph, implementing the baseline of
//! *A Complete Anytime Algorithm for Treewidth* (Gogate & Dechter, UAI 2004):
//! base branch-and-bound (Figure 4) with the minor-min-width lower bound
//! (Figure 1) and a single-pass deterministic min-fill upper bound. None of
//! the enhancement rules from §5–6 are included.
//!
//! The graph representation is built on [`petgraph::graph::UnGraph`] with
//! logical (masked) vertex deletion, so `NodeIndex` values stay stable across
//! elimination operations.
//!
//! # Layout
//!
//! - [`graph`] — the core [`Graph`] data structure.
//! - [`algorithm`] — QuickBB ([`treewidth`]) and its bounds
//!   ([`min_fill`][algorithm::min_fill], [`minor_min_width`][algorithm::mmw]).
//! - [`generator`] — named graphs ([`path`][generator::named::path],
//!   [`cycle`][generator::named::cycle], [`clique`][generator::named::clique],
//!   …) and random graph models
//!   ([`gnp`][generator::random::gnp], [`gnm`][generator::random::gnm],
//!   [`random_regular`][generator::random::random_regular],
//!   [`barabasi_albert`][generator::random::barabasi_albert],
//!   [`random_graph`][generator::random::random_graph]).
//!
//! For convenience, [`treewidth`] and [`Graph`] are re-exported at the crate
//! root.
//!
//! # Example
//! ```
//! use cartographer::graph::Graph;
//! use cartographer::treewidth;
//!
//! // Path 0-1-2-3-4, treewidth 1.
//! let g = Graph::from_edges([(0,1),(1,2),(2,3),(3,4)]);
//! let r = treewidth(&g);
//! assert_eq!(r.treewidth, 1);
//! assert_eq!(r.order.len(), 5);
//! ```

pub mod algorithm;
pub mod generator;
pub mod graph;

mod cli;

pub use algorithm::{BbResult, treewidth};
pub use graph::Graph;
