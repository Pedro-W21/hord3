use std::simd::{cmp::SimdPartialOrd, num::SimdInt, Mask, Simd};

use crate::defaults::default_rendering::vectorinator_binned::{consts::TABLE_U8_F32, triangles::{collux_u8_to_f32_simd, color_u32_to_u8_simd, zero_to_one_f32_simd_to_0_to_255_u8_simd}};

#[test]
pub fn large_simd_test() {
    let mut cool_vector = Simd::from_array([0.0_f32 ; 32]);
    let mut other_cool_vector = Simd::from_array([1.0_f32 ; 32]);
    cool_vector += other_cool_vector;
    dbg!(cool_vector);
}

#[test]
pub fn comparisons_test() {
    let mut cool_vector = Simd::from_array([1.0, -1.0, 1.0, 1.0]);
    assert_eq!(cool_vector.simd_ge(Simd::splat(0.0)), Mask::from_array([true, false, true, true]));
    let mut other_cool_vector = Simd::from_array([-1.0, 1.0, -1.0, 1.0]);
    assert_eq!(other_cool_vector.simd_ge(Simd::splat(0.0)) & cool_vector.simd_ge(Simd::splat(0.0)), Mask::from_array([false, false, false, true]));
}

#[test]
pub fn f32_u8_conv_test() {
    let mut les_cols = Simd::from_array([0, 50, 255, 100]);
    let mut cool_vector = Simd::from_array([TABLE_U8_F32[0],TABLE_U8_F32[50],TABLE_U8_F32[255],TABLE_U8_F32[100]]);
    unsafe {
        let mut colors = zero_to_one_f32_simd_to_0_to_255_u8_simd(cool_vector);
        dbg!(cool_vector, colors, les_cols);
        assert_eq!(les_cols, colors);
    }
    unsafe {
        let mut les_r = Simd::from_array([25, 10, 200, 250]);
        let mut les_g = Simd::from_array([100, 150, 40, 230]);
        let mut les_b = Simd::from_array([200, 100, 50,60]);
        // assert_eq!(collux_u8_to_f32_simd(color_u32_to_u8_simd((les_r.cast(), les_g.cast(), les_b.cast()))), collux_u8_to_f32_simd(color_u8));
        // let mut colors = collux_u8_to_f32_simd((les_cols, les_cols, les_cols));
    }
}