use proptest::prelude::*;

/// Strategy generating arbitrary byte buffers for fuzzer-style payload testing.
/// Includes empty buffers, small fragments, large buffers, and boundary sizes around 256KB.
pub fn arbitrary_payload_bytes() -> impl Strategy<Value = Vec<u8>> {
    prop_oneof![
        // Empty
        Just(Vec::new()),
        // Short random bytes
        prop::collection::vec(any::<u8>(), 1..64),
        // Random UTF-8 text / JSON-ish
        "[ -~]{0,256}".prop_map(|s| s.into_bytes()),
        // Malformed JSON structures
        prop_oneof![
            Just(b"{\"session_id\":".to_vec()),
            Just(b"{\"hook_event_name\":\"PreToolUse\"".to_vec()),
            Just(b"{\"hook_event_name\": 12345}".to_vec()),
            Just(b"\x00\xff\xfe\xfd".to_vec()),
            Just(vec![b'{'; 100]),
            Just(vec![b'}'; 100]),
        ],
        // Medium random payload
        prop::collection::vec(any::<u8>(), 256..4096),
        // Payload at hook cap boundary (256 KB = 262144 bytes)
        Just(vec![b'A'; 262_144]),
        // Payload exceeding hook cap (256 KB + 1 byte)
        Just(vec![b'B'; 262_145]),
    ]
}

/// Strategy generating syntactically valid JSON with arbitrary schema noise.
pub fn noisy_json_payload() -> impl Strategy<Value = Vec<u8>> {
    (
        proptest::option::of("[a-z0-9_-]{1,32}"),
        proptest::option::of("[A-Za-z0-9]{1,32}"),
        "[a-z0-9_]{1,16}",
        "[a-z0-9_]{1,16}",
    )
        .prop_map(|(session_id, hook_event, extra_key, extra_val)| {
            let mut obj = serde_json::Map::new();
            if let Some(sid) = session_id {
                obj.insert("session_id".into(), serde_json::Value::String(sid));
            }
            if let Some(event) = hook_event {
                obj.insert("hook_event_name".into(), serde_json::Value::String(event));
            }
            obj.insert(extra_key, serde_json::Value::String(extra_val));
            serde_json::to_vec(&obj).unwrap_or_default()
        })
}
