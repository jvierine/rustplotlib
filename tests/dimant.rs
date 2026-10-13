#[path = "../examples/support/dimant.rs"]
mod dimant;
#[test]
fn matches_sabmod_reference_and_quadrature_refinement() {
    let a = dimant::Density::new(64);
    let b = dimant::Density::new(128);
    // Compare quadrature refinement and cylindrical symmetry.
    for [z, r] in [
        [-5., 0.],
        [5., 0.],
        [0., 1.],
        [1., 1.],
        [-1., 1.],
        [0.01, 0.01],
        [15., 8.],
        [2., 0.0001],
    ] {
        let x = a.at(z, r);
        let y = b.at(z, r);
        assert!((x - y).abs() / y.abs().max(1e-12) < 1e-8);
        assert!(x > 0. && x.is_finite());
        assert!((a.at(z, -r) - x).abs() < 1e-12);
    }
    // SciPy quad reference values from the supplied sabmod implementation.
    assert!((a.at(0.0, 1.0) - 3.60451763844141015e-01).abs() < 1e-9);
    assert!((a.at(1.0, 1.0) - 6.60477301201982780e-01).abs() < 1e-9);
    assert!((a.at(-1.0, 1.0) - 8.11663432278448144e-02).abs() < 1e-9);
    assert!((a.at(0.01, 0.01) - 1.40280938774513658e+02).abs() < 1e-9);
    assert!((a.at(15.0, 8.0) - 8.83254500298491334e-03).abs() < 1e-9);
    assert!(a.at(0., 0.).is_nan());
}
