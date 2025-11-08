//! Show “realistic” 4D polytopes for quick visual sanity on counts.
//!
//! Usage:
//!   cargo run -p viterbo --example realistic_polys -- vertices
//!   cargo run -p viterbo --example realistic_polys -- faces
//!
//! Prints a few samples with (V,H) counts:
//! - vertices mode: aims for V in [5,25]
//! - faces mode: aims for H in [5,10]
//!
//! Ticket: 8ed3-2d-4d-generators

use viterbo::rand4::{
    PolytopeGenerator4, RandomFacesGenerator, RandomFacesParams, RandomVerticesGenerator,
    RandomVerticesParams,
};

fn main() {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "vertices".to_string());
    match mode.as_str() {
        "vertices" => show_vertices_mode(),
        "faces" => show_faces_mode(),
        _ => {
            eprintln!("usage: realistic_polys [vertices|faces]");
        }
    }
}

fn show_vertices_mode() {
    let params = RandomVerticesParams {
        vertices_min: 5,
        vertices_max: 25,
        radius_min: 0.5,
        radius_max: 1.5,
        anisotropy: None,
        max_attempts: 10,
    };
    let mut gen = RandomVerticesGenerator::new(params, 2025).unwrap();
    for i in 0..5 {
        let s = gen.generate_next().unwrap().unwrap();
        println!(
            "vertices sample {i}: V={}, H={}",
            s.polytope.v.len(),
            s.polytope.h.len()
        );
    }
}

fn show_faces_mode() {
    let params = RandomFacesParams {
        facets_min: 5,
        facets_max: 10,
        radius_min: 0.4,
        radius_max: 1.2,
        anisotropy: None,
        max_attempts: 20,
    };
    let mut gen = RandomFacesGenerator::new(params, 777).unwrap();
    for i in 0..5 {
        let s = gen.generate_next().unwrap().unwrap();
        println!(
            "faces sample {i}: V={}, H={}",
            s.polytope.v.len(),
            s.polytope.h.len()
        );
    }
}
