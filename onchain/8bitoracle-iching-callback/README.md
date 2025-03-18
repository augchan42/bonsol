# 8BitOracle I Ching Callback Program

This Solana program receives and stores I Ching hexagram readings from the Bonsol ZK program. It demonstrates how to implement a callback program that persists ZK program outputs on-chain.

## Overview

The program:
1. Receives verified outputs from the Bonsol protocol
2. Parses hexagram data (lines and ASCII art)
3. Stores readings in Program Derived Addresses (PDAs)

## Program ID Verification

Before deploying the program, you must ensure the program ID in the source code matches your keypair:

1. Get your program ID:
```bash
solana-keygen pubkey scripts/program-keypair.json | cat
```

2. Update the program ID in `src/lib.rs`:
```rust
// Update this line with your program ID
solana_program::declare_id!("your_program_id_here");
```

3. Build and deploy:
```bash
cargo build-sbf
solana program deploy target/deploy/bitoracle_iching_callback.so
```

> ⚠️ **Important**: The program ID in `lib.rs` MUST match your keypair's program ID, or deployment will fail. This is a required manual step that cannot be automated.

## Program Derived Addresses (PDAs)

### What are PDAs?
PDAs are special Solana accounts that are:
- Deterministically derived from seeds (like an execution account)
- Owned and controlled by the program
- Cannot have a private key (off the Ed25519 curve)
- Perfect for storing program-specific data

### Our PDA Structure
For each I Ching reading, we create a PDA using:
```rust
Seeds:
- "8bitoracle-hexagram" (prefix)
- "v1" (version)
- execution_account.key (the account that requested the reading)
```

This ensures:
- Each reading has a unique storage location
- Anyone can find a reading's data if they know the execution account
- Only our program can modify the stored data

### Stored Data Structure
Each PDA stores:
```rust
pub struct HexagramData {
    pub lines: [u8; 6],         // The 6,7,8,9 values for each line
    pub ascii_art: String,      // The ASCII representation
    pub timestamp: i64,         // When the reading was done
    pub is_initialized: bool,    // To check if the account is initialized
}
```

## Development

### Prerequisites
- Rust and Solana CLI tools
- Node.js and npm (for helper scripts)
- TypeScript (for PDA derivation script)

### Building
```bash
# Build the program
cargo build-sbf

# Install script dependencies
cd scripts && npm install
```

### Testing
```bash
# Run program tests
cargo test-sbf

# Test PDA derivation
cd scripts && npm run derive-pda <program_id> <execution_account>
```

### Helper Scripts
The `scripts` directory contains:
- `derive-pda.ts`: TypeScript script to calculate PDAs
- `package.json` & `tsconfig.json`: TypeScript configuration

## Usage

### 1. Deploy Program
After verifying the program ID (see [Program ID Verification](#program-id-verification)):
```bash
solana program deploy target/deploy/bitoracle_iching_callback.so
```

### 2. Execute I Ching Reading
```bash
# From project root
./images/8bitoracle-iching/scripts/03-generate-input-with-callback.sh
./images/8bitoracle-iching/scripts/04-execute.sh
```

## Account Structure

### Required Accounts
1. Execution Account (signer)
   - The account that requested the I Ching reading
   - Must sign the transaction
   - Used in PDA derivation

2. Hexagram Storage Account (PDA)
   - Stores the hexagram data
   - Derived from execution account
   - Created on first use

3. System Program
   - Used for creating new accounts

## Security Considerations

- Only the program can modify PDA data
- PDAs are verified before use
- Account validation ensures:
  - Execution account is a signer
  - PDA derivation is correct
  - System program is legitimate

## Error Handling

The program includes custom errors:
```rust
pub enum CallbackError {
    InvalidInstruction,
    NotRentExempt,
    InvalidHexagramData,
    InvalidPDA,
}
```

## Development Mode

When using the local validator:
```bash
# Start validator with program
./bin/validator.sh -r

# Generate input with callback
./images/8bitoracle-iching/scripts/03-generate-input-with-callback.sh
``` 