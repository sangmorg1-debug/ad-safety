// Test: Rule 2 coroutine rejection check
#![feature(autodiff, coroutines, coroutine_trait, stmt_expr_attributes)]
use std::autodiff::autodiff_reverse;
use std::ops::{Coroutine, CoroutineState};
use std::pin::Pin;

#[autodiff_reverse(df_invalid_coroutine, Active, Active)]
fn f_invalid_coroutine(x: f32) -> f32 {
    let mut coroutine = #[coroutine] move || {
        let val = x;
        yield val;
        yield val * 3.0;
    };
    
    let mut pin = Pin::new(&mut coroutine);
    let mut sum = 0.0;
    match pin.as_mut().resume(()) {
        CoroutineState::Yielded(v) => sum += v,
        CoroutineState::Complete(_) => {}
    }
    sum
}
