// Test: Valid autodiff signature using Duplicated reference parameters
#![feature(autodiff)]
use std::autodiff::autodiff_reverse;

#[autodiff_reverse(df_valid, Duplicated, Duplicated, Active)]
fn f_valid<'a>(x: &'a f32, y: &'a f32) -> f32 {
    *x * *y
}
