//! L1 Property Tests: Adapter Input Normalization & Fuzzing
//!
//! Requirement: MYR-HARN-003
//! Arbitrary byte payloads delivered to agent hook parsers must never cause a panic.
//! They must deterministically yield either a well-formed NormalizedEvent
//! or a typed AdapterError. Oversized payloads (>256KB) must be rejected immediately.

use agentpet_adapters::{claude_code, codex, AdapterError, MAX_HOOK_PAYLOAD_BYTES};
use agentpet_harness::generators::adapters::{arbitrary_payload_bytes, noisy_json_payload};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_claude_parser_never_panics_on_arbitrary_input(payload in arbitrary_payload_bytes()) {
        fuzz_adapter_parser(claude_code::parse_hook_payload, &payload, false);
    }

    #[test]
    fn prop_codex_parser_never_panics_on_arbitrary_input(payload in arbitrary_payload_bytes()) {
        fuzz_adapter_parser(codex::parse_hook_payload, &payload, false);
    }

    #[test]
    fn prop_claude_parser_handles_noisy_json(payload in noisy_json_payload()) {
        fuzz_adapter_parser(claude_code::parse_hook_payload, &payload, false);
    }

    #[test]
    fn prop_codex_parser_handles_noisy_json(payload in noisy_json_payload()) {
        fuzz_adapter_parser(codex::parse_hook_payload, &payload, false);
    }
}

fn fuzz_adapter_parser<F>(parser: F, payload: &[u8], mutate_simulate_panic: bool)
where
    F: Fn(&[u8]) -> Result<agentpet_adapters::NormalizedEvent, AdapterError>
        + std::panic::RefUnwindSafe,
{
    let parse_result = std::panic::catch_unwind(|| {
        if mutate_simulate_panic {
            panic!("Simulated unhandled parser crash on hostile input");
        }
        parser(payload)
    });

    // Invariant 1: Parser must never panic
    assert!(
        parse_result.is_ok(),
        "ADAPTER PANICKED on input of length {}",
        payload.len()
    );

    let result = parse_result.unwrap();

    // Invariant 2: Payloads strictly exceeding the 256KB limit must be rejected as PayloadTooLarge
    if payload.len() > MAX_HOOK_PAYLOAD_BYTES {
        assert!(
            matches!(result, Err(AdapterError::PayloadTooLarge)),
            "Oversized payload ({} bytes) was not rejected with PayloadTooLarge: {:?}",
            payload.len(),
            result
        );
        return;
    }

    // Invariant 3: If parsed successfully, event must have non-empty identifiers
    if let Ok(event) = result {
        assert!(
            !event.event_id.trim().is_empty(),
            "Normalized event has empty event_id"
        );
        assert!(
            !event.agent_id.trim().is_empty(),
            "Normalized event has empty agent_id"
        );
    }
}

#[test]
fn mutation_sanity_check_fails_if_parser_panics() {
    let payload = b"not json at all \xff\xfe";
    let panic_detected = std::panic::catch_unwind(|| {
        fuzz_adapter_parser(claude_code::parse_hook_payload, payload, true);
    });

    assert!(
        panic_detected.is_err(),
        "Mutation sanity check failed: simulated parser panic was not caught!"
    );
}
