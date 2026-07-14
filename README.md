# ad-safety

`ad-safety` is a static safety auditor and compiler plugin for Rust's experimental automatic differentiation feature (`#![feature(autodiff)]`). It acts as a MIR-level auditor that prevents known compiler-crash (ICE) and silent-corruption traps before they propagate to the LLVM/Enzyme backend.

---

## 🛡️ Differentiating Safely: Rules Enforced

Rust's autodiff integration is currently experimental and pre-stabilization. `ad-safety` intercepts code matching two critical error patterns:

### Rule 1: ICE Prevention on Active Reference/Pointer Parameters
* **The Problem:** Differentiating reference parameters (`&f32`) or pointers (`*mut f32`) marked as `Active` in reverse mode crashes the compiler during LLVM/Enzyme codegen (`invertPointerM` assertion failure).
* **The Diagnostic:** `ad-safety` catches these active references and aborts with a helpful suggestion showing how to rewrite them using `Duplicated` to calculate gradients in-place:
  ```
  error: parameter 1 has type `&'a f32` but is marked as `Active` in reverse-mode autodiff
   --> src/main.rs:8:1
    |
  8 | #[autodiff_reverse(df, Active, Active, Active)]
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ differentiated here
    |
    = help: reference and pointer parameters must be marked as `Duplicated` (not `Active`) in reverse mode. Change the activity to `Duplicated` and pass a shadow parameter to receive the gradient in-place.
  ```

#### Crash-Prone Pattern:
```rust
#[autodiff_reverse(df, Active, Active, Active)]
fn f(x: &f32, y: &f32) -> f32 { *x * *y } // ICE: invertPointerM
```

#### Safe Workaround:
```rust
#[autodiff_reverse(df, Duplicated, Duplicated, Active)]
fn f(x: &f32, y: &f32) -> f32 { *x * *y } // Compiles & executes cleanly!

// Called as:
let mut dx = 0.0;
let mut dy = 0.0;
let val = df(&x, &mut dx, &y, &mut dy, 1.0);
```

### Rule 2: Silent Gradient Corruption on Coroutines/Generators
* **The Problem:** Differentiating through custom structure state-machines like coroutines/generators compiles **without any errors or warnings**, but silently drops gradient tracking because the compiler fails to emit field-level type-tree/activity metadata. This yields incorrect gradients (`dx: 0` instead of the correct value).
* **The Diagnostic:** `ad-safety` traverses the MIR call graph of the differentiated function and rejects any usage of coroutine state types.
  ```
  error: use of coroutines/generators is not supported in differentiated functions
    --> src/main.rs:15:9
     |
  14 | fn f(x: f32) -> f32 {
     | ------------------- inside this differentiated function
  15 |     let mut coroutine = #[coroutine] move || { ... };
     |         ^^^^^^^^^^^^^
     |
     = note: differentiating through coroutine state machines silently ignores state mutation, leading to incorrect gradients (dx: 0)
  ```

---

## 📦 Workspace Architecture

The workspace consists of three crates:
1. `ad-safety-core`: The analysis library that registers as a `rustc_driver::Callbacks` plugin.
2. `cargo-ad-safety`: The cargo subcommand command-line interface. It runs `cargo check` under the hood and automatically configures `RUSTC_WORKSPACE_WRAPPER` to wrap compilation commands for workspace crates only.
3. `ad-safety-tests`: An automated integration test suite containing our UI regressions and snapshot tests.

---

## 🚀 Installation & Usage

### Prerequisites
* Rust Nightly (`rustc-dev` and `llvm-tools-preview` components installed):
  ```bash
  rustup +nightly component add rustc-dev llvm-tools-preview
  ```

### Build & Install
Build the workspace using `RUSTC_BOOTSTRAP=1` (required to build compiler private libraries):
```bash
RUSTC_BOOTSTRAP=1 cargo build --release
cargo install --path cargo-ad-safety
```

### Running the Auditor
Run the auditor on your Cargo package or workspace:
```bash
RUSTC_BOOTSTRAP=1 RUSTFLAGS="-Z autodiff=Enable" cargo ad-safety
```

---

## 🧪 Snapshot Testing

We use a snapshot-based test runner under `ad-safety-tests` which compiles UI test cases in `tests/ui/` and asserts their compiler errors against checked-in `.stderr` snapshots.

To run the test suite:
```bash
RUSTC_BOOTSTRAP=1 cargo test
```

To update the expected diagnostic outputs after updating the linter rules:
```bash
UPDATE_EXPECT=1 RUSTC_BOOTSTRAP=1 cargo test
```
