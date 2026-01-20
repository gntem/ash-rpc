#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Try to parse arbitrary bytes as JSON-RPC Request
    // This tests the robustness of Request deserialization
    let _ = serde_json::from_slice::<ash_rpc_core::Request>(data);
});
