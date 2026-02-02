#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    code: i32,
    message: String,
    data_raw: Vec<u8>,
    use_data: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        let mut builder = ash_rpc::ErrorBuilder::new(input.code, &input.message);

        if input.use_data {
            if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&input.data_raw) {
                builder = builder.data(data);
            }
        }

        let error = builder.build();

        let _ = serde_json::to_string(&error);

        if let Ok(json) = serde_json::to_vec(&error) {
            let _ = serde_json::from_slice::<ash_rpc::Error>(&json);
        }

        let _ = error.is_parse_error();
        let _ = error.is_invalid_request();
        let _ = error.is_method_not_found();
        let _ = error.is_invalid_params();
        let _ = error.is_internal_error();
        let _ = error.is_server_error();
    }
});
