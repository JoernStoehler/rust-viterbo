//! 4D Convex Polytopes (H- and V-representations; explicit, simple algorithms).
//!
//! Purpose
//! - Production module for 4D polytopes where counts are moderate (≈1e6). We run
//!   fewer modifying ops, so we prioritize clarity and explicit conversions.
//!
//! Why this design
//! - Track both H‑ and V‑rep (either may be empty until requested).
//! - Keep conversions explicit (enumeration), dependency‑light, and easy to audit.
//! - Make functions accept only what they need (don’t force “rich” objects).
//!
//! References
//! - TH: docs/src/thesis/geom4d_polytopes.md
//! - AGENTS: `AGENTS.md`
//! - Related code: `crate::geom2` for 2D mappings

pub(crate) mod cfg;
mod types;
mod util;
mod convert;
mod faces;
mod maps;

pub use types::{Hs4, Poly4};
pub use faces::{Face1, Face2, Face3, enumerate_faces_from_h};
pub use maps::{
    face2_as_poly2_hrep, is_symplectic, j_matrix_4, invert_affine_4, oriented_orth_map_face2,
    reeb_on_edges_stub, reeb_on_facets,
};

