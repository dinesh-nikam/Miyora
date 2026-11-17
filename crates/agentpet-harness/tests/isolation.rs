//! Production Isolation Guard
//!
//! Requirement: MYR-HARN-006
//! Asserts that agentpet-harness is strictly a test-only crate and is never
//! linked into the dependency trees of release binaries (agentpet-cli, agentpet-desktop).

use std::process::Command;

#[test]
fn test_harness_is_never_linked_into_production_crates() {
    for package in ["agentpet-cli", "agentpet-desktop"] {
        let output = Command::new("cargo")
            .args(["tree", "-p", package])
            .output()
            .expect("cargo tree runs");

        assert!(
            output.status.success(),
            "cargo tree failed for package {package}"
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("agentpet-harness"),
            "CRITICAL VIOLATION: agentpet-harness is leaked into production dependency tree of {package}!"
        );
    }
}
