use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_ui_suite() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    
    // Path to the cargo-ad-safety binary
    // When running tests, cargo compiles binaries to target/debug/
    let binary_path = manifest_dir
        .parent()
        .unwrap()
        .join("target")
        .join("debug")
        .join("cargo-ad-safety");

    assert!(binary_path.exists(), "cargo-ad-safety binary not found at {:?}", binary_path);

    let test_cases = vec![
        ("rule1_active_ref", false),
        ("rule2_coroutine", false),
        ("valid", true),
    ];

    let temp_dir = env::temp_dir().join("ad_safety_ui_tests");
    fs::create_dir_all(&temp_dir).unwrap();

    for (case_name, should_succeed) in test_cases {
        let rs_path = manifest_dir.join("tests").join("ui").join(format!("{}.rs", case_name));
        let stderr_path = rs_path.with_extension("stderr");

        // Run cargo-ad-safety as a compiler directly
        let mut cmd = Command::new(&binary_path);
        
        // Pass "rustc" as first argument so cargo-ad-safety runs as a wrapper
        cmd.arg("rustc");
        cmd.arg(&rs_path);
        cmd.arg("--crate-type=lib");
        cmd.arg("-Z");
        cmd.arg("autodiff=Enable");
        cmd.arg("--edition=2021");
        cmd.arg("--out-dir");
        cmd.arg(&temp_dir);

        // Set RUSTC_BOOTSTRAP=1 to allow rustc_private compiler crates
        cmd.env("RUSTC_BOOTSTRAP", "1");
        
        // Ensure the dynamic linker can find compiler library paths
        if env::var("LD_LIBRARY_PATH").is_err() {
            cmd.env("LD_LIBRARY_PATH", "/home/fromi/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib");
        }

        let output = cmd.output().expect("failed to execute cargo-ad-safety compiler driver");

        if should_succeed {
            assert!(
                output.status.success(),
                "Expected {} to compile successfully, but it failed.\nStderr:\n{}",
                case_name,
                String::from_utf8_lossy(&output.stderr)
            );
        } else {
            assert!(
                !output.status.success(),
                "Expected {} to fail compilation, but it succeeded.",
                case_name
            );

            // Clean up and normalize stderr output for snapshot comparison
            let raw_stderr = String::from_utf8_lossy(&output.stderr);
            let mut normalized_stderr = String::new();

            for line in raw_stderr.lines() {
                // Filter out non-deterministic lines like compiler time/performance or panic backtraces
                if line.contains("Finished") || line.contains("Checking") || line.contains("warning: ") || line.contains("dead_code") {
                    continue;
                }
                
                // Replace any absolute workspace paths with a generic one
                let line_normalized = line.replace(manifest_dir.to_str().unwrap(), "$WORKSPACE");
                normalized_stderr.push_str(&line_normalized);
                normalized_stderr.push('\n');
            }
            
            let normalized_stderr = normalized_stderr.trim().to_string();

            // If UPDATE_EXPECT env var is set, update the snapshot file
            if env::var("UPDATE_EXPECT").unwrap_or_default() == "1" {
                fs::write(&stderr_path, &normalized_stderr).unwrap();
                println!("Updated snapshot for {}", case_name);
            } else {
                // Check if snapshot exists
                if !stderr_path.exists() {
                    // Create it initially if it doesn't exist
                    fs::write(&stderr_path, &normalized_stderr).unwrap();
                    println!("Created initial snapshot for {}", case_name);
                } else {
                    let expected_stderr = fs::read_to_string(&stderr_path)
                        .expect("failed to read expected stderr snapshot")
                        .trim()
                        .to_string();

                    assert_eq!(
                        normalized_stderr,
                        expected_stderr,
                        "Snapshot mismatch for UI test case '{}'. Run with UPDATE_EXPECT=1 to update snapshots.\n\nActual Stderr:\n{}\n\nExpected Stderr:\n{}",
                        case_name,
                        normalized_stderr,
                        expected_stderr
                    );
                }
            }
        }
    }
}
