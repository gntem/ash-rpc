#![no_main]

use libfuzzer_sys::fuzz_target;
use ash_rpc_core::sanitization::{SimplePattern, PatternTransform};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 || data.len() > 256 {
        return;
    }
    
    let split_point = data.len() / 2;
    let pattern = String::from_utf8_lossy(&data[..split_point]);
    let input = String::from_utf8_lossy(&data[split_point..]);
    
    if pattern.is_empty() {
        return;
    }
    
    let simple = SimplePattern::new(pattern.as_ref(), "[REDACTED]");
    let _ = simple.apply(input.as_ref());
});
