#![no_main]

use libfuzzer_sys::fuzz_target;
use ash_rpc_core::Request;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    
    let method = String::from_utf8_lossy(data);
    
    let req = Request::new(method.as_ref());
    let _ = req.expects_response();
    let _ = req.is_notification();
    
    if let Ok(json) = serde_json::to_string(&req) {
        let _: Result<Request, _> = serde_json::from_str(&json);
    }
});
