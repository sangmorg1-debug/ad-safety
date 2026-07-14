// Test: Rule 1 active reference parameter check
#![feature(autodiff)]
use std::autodiff::autodiff_reverse;

#[autodiff_reverse(df, Active, Active, Active)]
fn f_invalid<'a>(x: &'a f32, y: &'a f32) -> f32 {
    *x * *y
}
