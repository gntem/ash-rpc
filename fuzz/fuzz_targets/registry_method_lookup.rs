#![no_main]

use ash_rpc::MethodRegistry;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let method_name = String::from_utf8_lossy(data);

    let registry = MethodRegistry::empty();
    let _ = registry.has_method(&method_name);
    let _ = registry.get_methods();
    let _ = registry.method_count();
});
