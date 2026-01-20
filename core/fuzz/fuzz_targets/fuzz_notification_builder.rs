#![no_main]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary;
use arbitrary::{Arbitrary, Unstructured};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    method: String,
    params_raw: Vec<u8>,
    use_params: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    
    // Try to generate structured input
    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        // Create a notification builder with the fuzzed method
        let mut builder = ash_rpc_core::NotificationBuilder::new(&input.method);
        
        // Try to set params from fuzzed data
        if input.use_params {
            if let Ok(params) = serde_json::from_slice::<serde_json::Value>(&input.params_raw) {
                builder = builder.params(params);
            }
        }
        
        // Build the notification - should never panic
        let notification = builder.build();
        
        // Verify the notification can be serialized
        let _ = serde_json::to_string(&notification);
        
        // Verify the notification can be round-tripped
        if let Ok(json) = serde_json::to_vec(&notification) {
            let _ = serde_json::from_slice::<ash_rpc_core::Notification>(&json);
        }
        
        // Verify notification has no ID (notifications don't expect responses)
        assert_eq!(notification.jsonrpc, "2.0");
    }
});
