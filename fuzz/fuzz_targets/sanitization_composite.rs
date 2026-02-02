#![no_main]

use ash_rpc::sanitization::{CompositeTransform, PatternTransform, SimplePattern};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() < 3 || data.len() > 192 {
        return;
    }

    let chunk_size = data.len() / 3;
    let pattern1 = String::from_utf8_lossy(&data[..chunk_size]);
    let pattern2 = String::from_utf8_lossy(&data[chunk_size..chunk_size * 2]);
    let input = String::from_utf8_lossy(&data[chunk_size * 2..]);

    if pattern1.is_empty() || pattern2.is_empty() {
        return;
    }

    let composite = CompositeTransform::new()
        .add_transform(SimplePattern::new(pattern1.as_ref(), "[REDACTED1]"))
        .add_transform(SimplePattern::new(pattern2.as_ref(), "[REDACTED2]"));

    let _ = composite.apply(input.as_ref());
});
