//! Graph generators: deterministic named graphs and random graph models.
//!
//! - [`named`] provides the usual structured graphs (paths, cycles, cliques,
//!   grids, trees, …) as standalone functions returning [`Graph`].
//! - [`random`] provides random graph models — Erdős–Rényi `G(n, p)` and
//!   `G(n, m)`, random regular graphs, Barabási–Albert preferential
//!   attachment — plus a top-level [`random_graph`][random::random_graph]
//!   dispatcher that selects one at random.
//!
//! Each random generator accepts a `&mut impl Rng` so callers can plug in a
//! seeded RNG for reproducible tests.

pub mod named;
pub mod random;

pub use named::*;
pub use random::*;
