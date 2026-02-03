#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    method: String,
    params_raw: Vec<u8>,
    use_params: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        let mut builder = ash_rpc::NotificationBuilder::new(&input.method);

        if input.use_params {
            if let Ok(params) = serde_json::from_slice::<serde_json::Value>(&input.params_raw) {
                builder = builder.params(params);
            }
        }

        let notification = builder.build();

        let _ = serde_json::to_string(&notification);

        if let Ok(json) = serde_json::to_vec(&notification) {
            let _ = serde_json::from_slice::<ash_rpc::Notification>(&json);
        }

        assert_eq!(notification.jsonrpc, "2.0");
    }
});
