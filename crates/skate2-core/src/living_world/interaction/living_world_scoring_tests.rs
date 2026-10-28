use super::*;

#[test]
fn dmo_selection_uses_transformed_bounds_centre_and_horizontal_distance() {
    // Local box centred at X4, rotated so its world centre lies at Z6.
    let transform = [
        0., 0., -1., 0., 0., 1., 0., 0., 1., 0., 0., 0., 10., 20., 10., 1.,
    ];
    let min = [3., -1., -1., 0.];
    let max = [5., 1., 1., 0.];
    assert!(dmo_close_enough([10., 500., 6.], &transform, min, max).unwrap());
    assert!(dmo_close_enough([12., -500., 6.], &transform, min, max).unwrap());
    assert!(!dmo_close_enough([13., 20., 6.], &transform, min, max).unwrap());
    // Being at the entity origin alone does not pass this authored bounds test.
    assert!(!dmo_close_enough([10., 20., 10.], &transform, min, max).unwrap());
}
