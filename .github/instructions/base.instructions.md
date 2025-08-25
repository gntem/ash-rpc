---
applyTo: '**'
---
This is a Rust library that implements JSON RPC and support transports as features etc

# General behaviour guideline
Do not provide step by step feedback
Write a very brief summary of what were the changes.
do not comment, do not write documentation, do not write tests UNLESS EXPLICITLY asked for
If a package needs to be installed you need to ask for permission
DO NOT WRITE A README

# Code style
prefer builder patterns

# Packages description

- `core` is a package hosting traits, implementations, enums, all around the json rpc 
    - Core has features about tcp, http using axum middleware
- `stateful` is a package hosting traits, implementation, for consumers to implement json rpc services