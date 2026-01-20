#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as a vector of JSON-RPC Messages (batch)
    // This tests the robustness of batch message deserialization
    let _ = serde_json::from_slice::<Vec<ash_rpc_core::Message>>(data);
});
