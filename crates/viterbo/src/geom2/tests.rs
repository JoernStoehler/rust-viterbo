use super::*;
use nalgebra::{matrix, vector, Vector2};

#[test]
fn rotation_angle_pure_rotations_and_reflection() {
    // 90° rotation
    let th = std::f64::consts::FRAC_PI_2;
    let f = Aff2 {
        m: matrix![th.cos(), -th.sin(); th.sin(), th.cos()],
        t: vector![0.0, 0.0],
    };
    let rho = rotation_angle(&f).unwrap();
    assert!((rho - 0.5).abs() < 1e-12);
    // Identity
    let f_id = Aff2::identity();
    let rho0 = rotation_angle(&f_id).unwrap();
    assert!(rho0.abs() < 1e-12);
    // Shear (orientation-preserving)
    let sh = Aff2 {
        m: matrix![1.0, 1.0; 0.0, 1.0],
        t: vector![0.0, 0.0],
    };
    let rho_shear = rotation_angle(&sh).unwrap();
    assert!(rho_shear >= 0.0 && rho_shear <= 0.5 + 1e-12);
    // Reflection -> None
    let refl = Aff2 {
        m: matrix![-1.0, 0.0; 0.0, 1.0],
        t: vector![0.0, 0.0],
    };
    assert!(rotation_angle(&refl).is_none());
}

#[test]
fn strict_emptiness_detects_contradiction() {
    // x <= 0 and x >= 1 -> empty
    let mut p = Poly2::default();
    p.insert_halfspace(Hs2::new(vector![1.0, 0.0], 0.0));
    p.insert_halfspace(Hs2::new(vector![-1.0, 0.0], -1.0));
    assert!(matches!(
        p.halfspace_intersection(),
        HalfspaceIntersection::Empty
    ));
    // Unit box -> non-empty
    let mut q = Poly2::default();
    q.insert_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
    q.insert_halfspace(Hs2::new(vector![-1.0, 0.0], 0.0));
    q.insert_halfspace(Hs2::new(vector![0.0, 1.0], 1.0));
    q.insert_halfspace(Hs2::new(vector![0.0, -1.0], 0.0));
    assert!(matches!(
        q.halfspace_intersection(),
        HalfspaceIntersection::Bounded(_)
    ));

    // eps semantics: positive eps enlarges (empty remains empty), negative shrinks (may become empty)
    assert!(p.is_empty_eps(1e-9)); // still empty
    assert!(!q.is_empty_eps(-1e-6)); // shrunken unit box should still be non-empty
}

#[test]
fn aff1_compose_and_cut() {
    // A(z)= [2,3]·z + 1;  φ(z)= 2 I z + [1, -1]
    let a = Aff1 {
        a: vector![2.0, 3.0],
        b: 1.0,
    };
    let phi = Aff2 {
        m: nalgebra::Matrix2::identity() * 2.0,
        t: vector![1.0, -1.0],
    };
    let aphi = a.compose_with_affine2(&phi);
    // For z=[x,y], A(φ(z)) = [4,6]·z + (2*1 + 3*(-1) + 1) = [4,6]·z + 0
    assert!((aphi.a.x - 4.0).abs() < 1e-12 && (aphi.a.y - 6.0).abs() < 1e-12);
    assert!(aphi.b.abs() < 1e-12);
    // Cut at A_best = 5 => 4x + 6y <= 5
    let cut = aphi.to_cut(5.0);
    assert!((cut.n.x - 4.0).abs() < 1e-12 && (cut.n.y - 6.0).abs() < 1e-12);
    assert!((cut.c - 5.0).abs() < 1e-12);
}

#[test]
fn fixed_point_unique_and_line_cases() {
    let cfg = GeomCfg::default();
    // Unique: ψ(z) = 0.5 z + t, fixed point z* = 2 t
    let t = vector![0.2, -0.3];
    let psi = Aff2 {
        m: nalgebra::Matrix2::identity() * 0.5,
        t,
    };
    // Big box contains the solution
    let mut box_poly = Poly2::default();
    box_poly.insert_halfspace(Hs2::new(vector![1.0, 0.0], 10.0));
    box_poly.insert_halfspace(Hs2::new(vector![-1.0, 0.0], 10.0));
    box_poly.insert_halfspace(Hs2::new(vector![0.0, 1.0], 10.0));
    box_poly.insert_halfspace(Hs2::new(vector![0.0, -1.0], 10.0));
    let a = Aff1 {
        a: vector![1.0, 2.0],
        b: 0.0,
    };
    let (z, val) = fixed_point_in_poly(psi, &box_poly, &a, cfg).expect("unique fixed point");
    let z_star = t * 2.0;
    assert!((z - z_star).norm() < 1e-9);
    assert!((val - a.eval(z_star)).abs() < 1e-9);

    // Line case: (I−M) = diag(0, 0.5); t=(0, 1) -> x free, y fixed.
    let psi_line = Aff2 {
        m: matrix![1.0, 0.0; 0.0, 0.5],
        t: vector![0.0, 1.0],
    };
    // C: -1 <= x <= 1, no restriction on y; enforce y fixed by feasibility.
    let mut c_line = Poly2::default();
    c_line.insert_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
    c_line.insert_halfspace(Hs2::new(vector![-1.0, 0.0], 1.0));
    // A(z) = x; optimum at x = -1 along the feasible line.
    let a_x = Aff1 {
        a: vector![1.0, 0.0],
        b: 0.0,
    };
    let (z_line, val_line) =
        fixed_point_in_poly(psi_line, &c_line, &a_x, cfg).expect("line fixed points");
    assert!((z_line.x + 1.0).abs() < 1e-9);
    // y fixed by (1-0.5) y = 1 -> y = 2
    assert!((z_line.y - 2.0).abs() < 1e-9);
    assert!((val_line + 1.0).abs() < 1e-9);
}

#[test]
fn hull_to_strict_poly() {
    let points = vec![
        Vector2::new(0.0, 0.0),
        Vector2::new(1.0, 0.0),
        Vector2::new(1.0, 1.0),
        Vector2::new(0.0, 1.0),
    ];
    let p = from_points_convex_hull_strict(&points).unwrap();
    match p.halfspace_intersection() {
        HalfspaceIntersection::Bounded(verts) => assert!(verts.len() >= 4),
        _ => panic!("expected bounded"),
    }
}
