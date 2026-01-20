# Fuzzing Tests for ash-rpc-core

This directory contains fuzz tests for the `ash-rpc-core` library using `cargo-fuzz` and `libFuzzer`.

## Prerequisites

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

## Available Fuzz Targets

### 1. `fuzz_request_parse`

Tests the robustness of JSON-RPC Request deserialization by feeding arbitrary bytes to the parser.

### 2. `fuzz_response_parse`

Tests the robustness of JSON-RPC Response deserialization by feeding arbitrary bytes to the parser.

### 3. `fuzz_error_parse`

Tests the robustness of JSON-RPC Error deserialization by feeding arbitrary bytes to the parser.

### 4. `fuzz_batch_parse`

Tests the robustness of JSON-RPC BatchRequest deserialization by feeding arbitrary bytes to the parser.

## Running Fuzz Tests

### Run a specific fuzz target

```bash
# Run from the core/ directory
cargo fuzz run fuzz_request_parse

# Run with a specific timeout (e.g., 60 seconds)
cargo fuzz run fuzz_request_parse -- -max_total_time=60

# Run with specific number of jobs/threads
cargo fuzz run fuzz_request_parse -- -jobs=4
```

### Run all fuzz targets

```bash
# Run each target for a short duration
cargo fuzz run fuzz_request_parse -- -max_total_time=30
cargo fuzz run fuzz_response_parse -- -max_total_time=30
cargo fuzz run fuzz_error_parse -- -max_total_time=30
cargo fuzz run fuzz_batch_parse -- -max_total_time=30
```

## Understanding Results

- **Crashes**: If a fuzz target finds an input that causes a crash, it will be saved to `fuzz/artifacts/<target_name>/`
- **Coverage**: Fuzzing automatically tracks code coverage and tries to maximize it
- **Corpus**: Interesting test cases are saved to `fuzz/corpus/<target_name>/`

## Reproducing Crashes

If fuzzing finds a crash:

```bash
# Reproduce the crash
cargo fuzz run fuzz_request_parse fuzz/artifacts/fuzz_request_parse/crash-<hash>

# Debug the crash
cargo fuzz run fuzz_request_parse fuzz/artifacts/fuzz_request_parse/crash-<hash> -- -exact_artifact_path=crash
```

## Adding New Fuzz Targets

1. Create a new fuzz target file in `fuzz_targets/`:

   ```bash
   cargo fuzz add fuzz_new_feature
   ```

2. Implement the fuzzing logic in `fuzz_targets/fuzz_new_feature.rs`

3. Run the new target:

   ```bash
   cargo fuzz run fuzz_new_feature
   ```

## Next Steps

Future fuzz targets to add:

- Builder pattern fuzzing (RequestBuilder, ResponseBuilder)
- Sanitization module fuzzing
- Auth token validation fuzzing
- Registry method dispatch fuzzing
- Transport layer fuzzing (when features are enabled)

## Resources

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
