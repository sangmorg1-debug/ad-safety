// Touch dummy comment to run check with pretty printing
#![feature(autodiff, coroutines, coroutine_trait, stmt_expr_attributes)]
use std::autodiff::autodiff_reverse;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

// Violates Rule 1: active reference parameter in reverse mode
#[autodiff_reverse(df, Active, Active, Active)]
fn f_invalid<'a>(x: &'a f32, y: &'a f32) -> f32 {
    *x * *y
}

fn main() {}
