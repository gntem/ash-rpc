#![no_main]

use ash_rpc::sanitization::CaseInsensitivePattern;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 || data.len() > 100 {
        return;
    }

    let split_point = data.len() / 2;
    let pattern = String::from_utf8_lossy(&data[..split_point]);
    let input = String::from_utf8_lossy(&data[split_point..]);

    if pattern.is_empty() || pattern.len() > 10 {
        return;
    }

    let _ = CaseInsensitivePattern::new(pattern.as_ref(), "REDACTED");
});
