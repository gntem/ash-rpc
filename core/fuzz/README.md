# Fuzzing Tests

Fuzz tests for `ash-rpc-core` using cargo-fuzz.

## Setup

```bash
cargo install cargo-fuzz
```

## Targets

**Parsing**: `fuzz_request_parse`, `fuzz_response_parse`, `fuzz_error_parse`, `fuzz_batch_parse`

**Builders**: `fuzz_request_builder`, `fuzz_response_builder`, `fuzz_error_builder`, `fuzz_notification_builder`

## Usage

```bash
# Run a target
cargo fuzz run fuzz_request_parse

# Time limit
cargo fuzz run fuzz_request_parse -- -max_total_time=60

# Reproduce crash
cargo fuzz run fuzz_request_parse fuzz/artifacts/fuzz_request_parse/crash-<hash>
```

Crashes are saved to `fuzz/artifacts/<target>/`, corpus to `fuzz/corpus/<target>/`.
