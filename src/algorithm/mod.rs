//! Graph algorithms.
//!
//! Currently focused on **treewidth**: the QuickBB branch-and-bound exact
//! algorithm (Gogate & Dechter, UAI 2004) together with its building-block
//! upper bound ([min-fill][minfill]) and lower bound
//! ([minor-min-width][mmw]).
//!
//! [minfill]: minfill::min_fill
//! [mmw]: mmw::minor_min_width
//!
//! # Example
//! ```
//! use cartographer::generator::named::path;
//! use cartographer::algorithm::treewidth;
//!
//! // Path 0-1-2-3-4, treewidth 1.
//! let g = path(5);
//! let r = treewidth(&g);
//! assert_eq!(r.treewidth, 1);
//! assert_eq!(r.order.len(), 5);
//! ```

pub mod branchbound;
pub mod minfill;
pub mod mmw;

pub use branchbound::{BbResult, treewidth};
pub use minfill::min_fill;
pub use mmw::minor_min_width;
