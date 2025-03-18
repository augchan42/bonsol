# 乃ㄖ几丂ㄖㄥ
Bonsol is the Offchain compute framework to make everything possible on solana.

[![commitlint](https://github.com/bonsolcollective/bonsol/actions/workflows/commit-lint.yaml/badge.svg)](https://github.com/bonsolcollective/bonsol/actions/workflows/commit-lint.yaml)
[![Docker Build from Image CI](https://github.com/bonsolcollective/bonsol/actions/workflows/build-ci-image.yaml/badge.svg)](https://github.com/bonsolcollective/bonsol/actions/workflows/build-ci-image.yaml)

Interact with the docs at [Bonsol.sh](https://bonsol.sh)

# NOTE !!!!!
Do not use `node_keypair.json` in production, it is for local development only. 

## Requirements:
* Flat buffers 24.3.25
* For running the prover x86_64-linux due to (stark to snark tooling)[https://github.com/risc0/risc0/commit/7c6101f925e8fd1b3de09654941f0608d4459a2b]

## Program ID Verification and Setup
Before deploying the callback program, you must ensure the program ID in the source code matches your keypair:

1. Get your program ID from the keypair:
```bash
solana-keygen pubkey onchain/8bitoracle-iching-callback/scripts/program-keypair.json | cat
```

2. Open the callback program source:
```bash
code onchain/8bitoracle-iching-callback/src/lib.rs
# or use your preferred editor
```

3. Update the program ID in lib.rs:
- Find the line starting with `solana_program::declare_id!`
- Replace the existing program ID with your keypair's program ID
- Example:
```rust
// Update this line with your program ID
solana_program::declare_id!("2gPzr1AjyYT8JqAndyTDMDUsQsH8y3tc9CuKUtKA2Uv1");
```

4. Save lib.rs and rebuild the program:
```bash
cd onchain/8bitoracle-iching-callback
cargo build-sbf
```

**IMPORTANT**: The program ID in lib.rs MUST match your keypair's program ID, or deployment will fail. This step cannot be automated and must be done manually.

## Development vs Production Mode

### RISC0_DEV_MODE Behavior
The `RISC0_DEV_MODE` environment variable is a **compile-time** flag that affects how the Bonsol smart contract handles proof verification:

- When set during compilation:
  ```bash
  RISC0_DEV_MODE=1 cargo build-sbf  # Dev mode enabled
  ```
  The resulting program will always run in dev mode, skipping cryptographic verification.

- When not set during compilation:
  ```bash
  cargo build-sbf  # Production mode
  ```
  The resulting program will always require valid proofs.

**IMPORTANT**: Setting `RISC0_DEV_MODE=1` on the validator after deployment has no effect. The mode is determined at compile time and cannot be changed without recompiling and redeploying the program.

## Scripts and Configuration

### Running a Node (`bin/run-node.sh`)
The node runner script provides several options for running a Bonsol node with different configurations:

```bash
./bin/run-node.sh [-F cuda] [-L] [-d]
```

#### Options
- `-F cuda`: Enable CUDA support for GPU acceleration
- `-L`: Use local build instead of installed bonsol
- `-d`: Enable debug logging for all relevant modules

#### Debug Mode Features
When running with `-d`:
- Detailed logging for all components
- System configuration display
- Core dump configuration
- Build information

#### System Configuration
The script automatically configures:
- Unlimited stack size
- Unlimited virtual memory
- Unlimited max memory size
- Core dumps enabled (stored in `/tmp/cores`)
- Detailed system limits display

#### Log Levels
- `error`: Show errors only
- `warn`: Show warnings and errors
- `info`: Show general information (default)
- `debug`: Show detailed debugging information
- `trace`: Show all possible logging information

#### Debug Components
- `risc0_runner`: Image downloads, proofs, and claims
- `transaction_sender`: Transaction processing and status
- `input_resolver`: Input processing and validation
- `reqwest`: HTTP client logs
- `hyper`: Low-level HTTP details

## Roadmap
Stage 1: Dawn (current stage)
* Developer feedback
    * New features 
        * Interfaces
            * More Ingesters, Senders
            * More Input Types
        * Adding Integrations
            * Zktls,web proofs, client proving
    * Node Ops
        * Claim based prover network (SOL)
        * Prover Supply Integrations
* Community Building

## Contributing and Local Development 
Please see our [Contributing Guide](https://bonsol.sh/docs/contributing) for details on how to get started building 乃ㄖ几丂ㄖㄥ.
