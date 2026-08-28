pub mod zx;

mod utils;

pub use zx::fuse::fuse_total;
pub use zx::k3_remove_edge::k3_remove;
pub use zx::k3_remove_edge_ha_2::k3_remove_had2;
pub use zx::normalize_h_parity::normalize_h_parity_total;
pub use zx::reduce_had_triangle::reduce_had_triangle_total;
