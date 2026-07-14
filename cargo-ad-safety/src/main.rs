#![feature(rustc_private)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_session;

use rustc_session::EarlyDiagCtxt;
use rustc_session::config::ErrorOutputType;
use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    // Check if we are running as a cargo subcommand:
    // e.g. `cargo ad-safety`
    let is_cargo_subcommand = args.len() > 1 && args[1] == "ad-safety";

    if is_cargo_subcommand {
        // Get path to this binary to set as workspace wrapper
        let current_exe = env::current_exe().expect("failed to get current executable path");
        
        // Use CARGO env var if available, otherwise default to "cargo"
        let cargo_exe = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = Command::new(cargo_exe);
        cmd.arg("check");
        // Forward any extra arguments (like --all-targets, --verbose, etc.)
        if args.len() > 2 {
            cmd.args(&args[2..]);
        }
        cmd.env("RUSTC_WORKSPACE_WRAPPER", &current_exe);

        let status = cmd.status().expect("failed to run cargo check");
        ExitCode::from(status.code().unwrap_or(1) as u8)
    } else {
        // We are running as the rustc workspace wrapper.
        // Initialize logging and execute the compiler with our custom safety callbacks.
        let early_dcx = EarlyDiagCtxt::new(ErrorOutputType::default());
        rustc_driver::init_rustc_env_logger(&early_dcx);

        let mut callbacks = ad_safety_core::AdSafetyCallbacks;
        rustc_driver::catch_with_exit_code(|| {
            rustc_driver::run_compiler(&args[1..], &mut callbacks);
        })
    }
}
