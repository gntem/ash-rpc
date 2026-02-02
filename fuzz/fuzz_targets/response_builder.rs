#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    result_raw: Vec<u8>,
    error_code: i32,
    error_message: String,
    error_data_raw: Vec<u8>,
    id_raw: Vec<u8>,
    correlation_id: Option<String>,
    use_result: bool,
    use_error: bool,
    use_error_data: bool,
    use_id: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);

    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        let mut builder = ash_rpc::ResponseBuilder::new();

        if input.use_result {
            if let Ok(result) = serde_json::from_slice::<serde_json::Value>(&input.result_raw) {
                builder = builder.success(result);
            }
        }

        if input.use_error {
            let mut error = ash_rpc::Error::new(input.error_code, &input.error_message);

            if input.use_error_data {
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&input.error_data_raw)
                {
                    error = error.with_data(data);
                }
            }

            builder = builder.error(error);
        }

        if input.use_id {
            if let Ok(id) = serde_json::from_slice::<serde_json::Value>(&input.id_raw) {
                builder = builder.id(Some(id));
            }
        } else {
            builder = builder.id(None);
        }

        builder = builder.correlation_id(input.correlation_id);

        let response = builder.build();

        let _ = serde_json::to_string(&response);

        if let Ok(json) = serde_json::to_vec(&response) {
            let _ = serde_json::from_slice::<ash_rpc::Response>(&json);
        }

        if response.is_success() {
            assert!(response.result().is_some());
            assert!(response.error_info().is_none());
        }

        if response.is_error() {
            assert!(response.result().is_none());
            assert!(response.error_info().is_some());
        }
    }
});
