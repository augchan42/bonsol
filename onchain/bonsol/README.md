# Bonsol Solana Program

This directory contains the Bonsol Solana program (smart contract) that handles on-chain proof verification and state management.

## Structure

```
onchain/bonsol/
├── src/
│   ├── lib.rs           # Program entrypoint and module definitions
│   ├── proof_handling.rs # Core proof verification logic
│   ├── error.rs         # Custom error types
│   ├── prover.rs        # Prover constants and utilities
│   └── verifying_key.rs # ZK proof verification keys
├── Cargo.toml          # Program dependencies and configuration
└── README.md          # This file
```

## Overview

In Solana, smart contracts are called "programs". This program is responsible for:

1. Verifying zero-knowledge proofs from the Bonsol prover
2. Managing on-chain state
3. Processing Solana instructions (transactions)

## Development Mode

The program supports a development mode for faster testing:
- Set by `RISC0_DEV_MODE` environment variable during compilation
- **IMPORTANT**: This is a compile-time flag, not a runtime flag
- Once compiled and deployed, the dev mode setting cannot be changed by setting environment variables on the validator
- Skips cryptographic verification in dev mode
- Logs detailed verification steps for debugging

### Dev vs Production Builds

1. Development Build:
```bash
# Compile with dev mode enabled
RISC0_DEV_MODE=1 cargo build-sbf

# The resulting program will always run in dev mode,
# regardless of environment variables on the validator
```

2. Production Build:
```bash
# Compile without dev mode (production)
cargo build-sbf

# The resulting program will never run in dev mode,
# regardless of environment variables on the validator
```

## Building

Build the program using:
```bash
# Production build
cargo build-sbf

# Debug build with dev mode
RISC0_DEV_MODE=1 RUST_LOG=debug cargo build-sbf --verbose
```

Or use the project scripts:
```bash
# From project root
./images/8bitoracle-iching/scripts/01-build.sh --debug --rebuild-bonsol
```

## Testing

Run program tests with:
```bash
cargo test-sbf
```

## Local Development

### Running the Validator

The local test validator automatically builds and deploys the program with dev mode enabled:
```bash
# Start validator with default settings (recommended)
./bin/validator.sh

# Optional flags:
./bin/validator.sh -r  # Reset validator state
./bin/validator.sh -d  # Show additional debug output
```

The validator script:
- Builds the program with `RISC0_DEV_MODE=1` and debug logging
- Deploys to address `BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew`
- Configures proper logging for Solana program output
- Filters out noise from vote accounts

### Viewing Program Logs

While the validator is running, in a separate terminal:
```bash
solana logs | grep "Program log:"
```

## Deployment

The program is deployed to Solana using:
```bash
# From project root
./images/8bitoracle-iching/scripts/02-deploy.sh
```

## Key Components

### Proof Handling
- `proof_handling.rs` contains the core verification logic
- Supports both RISC0 v1.0.1 and v1.2.1 proof formats
- Implements detailed logging for debugging

### Error Handling
- Custom error types defined in `error.rs`
- Uses `thiserror` for ergonomic error definitions
- Proper error propagation throughout the program

### Dependencies
- Uses `ark-bn254` for BN254 curve operations
- `groth16-solana` for ZK proof verification
- `bonsol-interface` for shared types and constants

## Logging

Debug logs can be viewed by:
1. Building with `RUST_LOG=debug`
2. Running validator with appropriate logging flags
3. Using `solana logs` command to view program output

## Common Issues

1. **Proof Verification Fails**: Check if dev mode is enabled correctly
2. **Missing Logs**: Ensure proper `RUST_LOG` configuration
3. **Build Errors**: Make sure you're using `cargo build-sbf` for Solana programs 