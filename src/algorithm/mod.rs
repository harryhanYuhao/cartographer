//! Graph algorithms.
//!
//! Currently focused on **treewidth**: the QuickBB branch-and-bound exact
//! algorithm (Gogate & Dechter, UAI 2004) together with its building-block
//! upper bound ([min-fill][minfill]) and lower bound
//! ([minor-min-width][mmw]), plus the §5–6 enhancement rules, and Tamaki's
//! positive-instance driven dynamic programming algorithm
//! ([PID-BT][pidd::pidd_tw]).
//!
//! [minfill]: minfill::min_fill
//! [mmw]: mmw::minor_min_width
//!
//! # Example
//! ```
//! use cartographer::generator::named::path;
//! use cartographer::algorithm::bb;
//!
//! // Path 0-1-2-3-4, treewidth 1.
//! let g = path(5);
//! let r = bb(&g);
//! assert_eq!(r.treewidth, 1);
//! assert_eq!(r.order.len(), 5);
//! ```

pub mod branchbound;
pub mod local_comp;
pub mod minfill;
pub mod mmw;
pub mod pidd;

pub use branchbound::{BbResult, bb, bb_tw};
pub use minfill::min_fill;
pub use mmw::minor_min_width;
pub use pidd::{pidd_tw};
