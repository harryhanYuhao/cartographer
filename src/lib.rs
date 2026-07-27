//! QuickBB: a branch-and-bound algorithm for the treewidth of an undirected
//! graph.
//!
//! Implements the *minimal baseline* of *A Complete Anytime Algorithm for
//! Treewidth* (Gogate & Dechter, UAI 2004): base branch-and-bound (Figure 4)
//! with the minor-min-width lower bound (Figure 1) and a single-pass
//! deterministic min-fill upper bound. None of the enhancement rules from
//! §5–6 are included.
//!
//! The graph representation is built on [`petgraph::graph::UnGraph`] with
//! logical (masked) vertex deletion, so `NodeIndex` values stay stable across
//! elimination operations.
//!
//! # Example
//! ```
//! use quickbb::graph::Graph;
//! use quickbb::treewidth;
//!
//! // Path 0-1-2-3-4, treewidth 1.
//! let g = Graph::from_edges([(0,1),(1,2),(2,3),(3,4)]);
//! let r = treewidth(&g);
//! assert_eq!(r.treewidth, 1);
//! assert_eq!(r.order.len(), 5);
//! ```

pub mod branchbound;
pub mod graph;
pub mod minfill;
pub mod mmw;

pub use branchbound::{BbResult, treewidth};
pub use graph::Graph;
