#![no_main]

use ash_rpc::auth::ConnectionContext;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let mut ctx = ConnectionContext::new();

    let key = String::from_utf8_lossy(data);
    ctx.insert(key.to_string(), String::from("test_value"));
    ctx.insert(key.to_string(), 42u32);
    ctx.insert(key.to_string(), vec![1, 2, 3]);

    let _: Option<&String> = ctx.get(&key);
    let _: Option<&u32> = ctx.get(&key);
    let _: Option<&Vec<u8>> = ctx.get(&key);
});
