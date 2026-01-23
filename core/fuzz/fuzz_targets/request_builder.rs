#![no_main]

use libfuzzer_sys::fuzz_target;
use libfuzzer_sys::arbitrary;
use arbitrary::{Arbitrary, Unstructured};

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    method: String,
    params_raw: Vec<u8>,
    id_raw: Vec<u8>,
    correlation_id: Option<String>,
    use_params: bool,
    use_id: bool,
}

fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    
    if let Ok(input) = FuzzInput::arbitrary(&mut u) {
        let mut builder = ash_rpc_core::RequestBuilder::new(&input.method);
        
        if input.use_params {
            if let Ok(params) = serde_json::from_slice::<serde_json::Value>(&input.params_raw) {
                builder = builder.params(params);
            }
        }
        
        if input.use_id {
            if let Ok(id) = serde_json::from_slice::<serde_json::Value>(&input.id_raw) {
                builder = builder.id(id);
            }
        }
        
        if let Some(corr_id) = input.correlation_id {
            builder = builder.correlation_id(corr_id);
        }
        
        let request = builder.build();
        
        let _ = serde_json::to_string(&request);
        
        if let Ok(json) = serde_json::to_vec(&request) {
            let _ = serde_json::from_slice::<ash_rpc_core::Request>(&json);
        }
    }
});
