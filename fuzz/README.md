# Fuzzing Tests

Fuzz tests for `ash-rpc` using cargo-fuzz.

## Setup

```bash
cargo install cargo-fuzz
```

## Usage

```bash
# Run a target
cargo fuzz run request_parse

# Time limit
cargo fuzz run request_parse -- -max_total_time=60

# Reproduce crash
cargo fuzz run request_parse fuzz/artifacts/request_parse/crash-<hash>
```

Crashes are saved to `fuzz/artifacts/<target>/`, corpus to `fuzz/corpus/<target>/`.
