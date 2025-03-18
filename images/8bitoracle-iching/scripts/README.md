# 🎲 I Ching Oracle Scripts

> 💫 This directory contains scripts for building, deploying, and executing the I Ching Oracle Zero-Knowledge program.

## 📋 Prerequisites

1. `bonsol` installed and in your PATH
2. Rust and Cargo ([rustup.rs](https://rustup.rs))
3. AWS credentials (if not using local deployment)
4. Solana CLI tools
5. Local validator running (see [Validator Setup](#-validator-setup))
6. Callback program deployed (see [Callback Setup](#-callback-setup))

> ℹ️ **Note:** Run all commands from the project root directory (`~/forked-projects/bonsol` or equivalent)

## 🚀 Quick Start

1. Make scripts executable:
```bash
chmod +x images/8bitoracle-iching/scripts/*.sh
chmod +x bin/*.sh
```

2. Set up environment:
```bash
# Create .env file in images/8bitoracle-iching/
cat > images/8bitoracle-iching/.env << EOL
AWS_ACCESS_KEY_ID=your_access_key_here
AWS_SECRET_ACCESS_KEY=your_secret_key_here
AWS_REGION=us-east-1
BUCKET=8bitoracle
EOL

# Load environment variables
source images/8bitoracle-iching/.env
```

3. Build and deploy:
```bash
# First build the I Ching program
images/8bitoracle-iching/scripts/01-build.sh

# Start local validator (will rebuild onchain programs)
bin/validator.sh

# In a new terminal, deploy using locally built binaries (recommended with debug)
images/8bitoracle-iching/scripts/02-deploy.sh --local --debug
```

4. Generate input and execute:
```bash
# Generate input
images/8bitoracle-iching/scripts/03-generate-input-with-callback.sh

# Execute program (recommended with debug)
images/8bitoracle-iching/scripts/04-execute.sh --local --debug
```

## 🔄 Script Inner Workings

### 1. Build (`01-build.sh`)
The build process involves two parts:

#### Main Build Script
The `01-build.sh` script:
- Cleans ALL target directories (including onchain programs)
- Rebuilds the I Ching program
- Optionally rebuilds Bonsol workspace

```bash
# With --rebuild-bonsol flag
./01-build.sh --rebuild-bonsol

# Without workspace rebuild
./01-build.sh
```

#### Validator Setup
The validator script (`bin/validator.sh`):
- Rebuilds all onchain programs using `cargo build-sbf`
- Starts a local Solana validator
- Must be run after `01-build.sh` to ensure onchain programs are rebuilt

> ⚠️ **Important**: Run `01-build.sh` first, then start the validator to ensure onchain programs are properly rebuilt

### 2. Deploy (`02-deploy.sh`)
Deploys the built program:
```bash
# Use locally built binaries
./02-deploy.sh --local --debug  # Recommended: include --debug for detailed logs

# Use installed bonsol
./02-deploy.sh --debug
```

### 3. Input Generation (`03-generate-input-with-callback.sh`)
Sets up program execution parameters:
- Configures callback program
- Initializes storage PDA
- Generates execution input

### 4. Execute (`04-execute.sh`)
Runs the program and processes results:
```bash
# Use locally built binaries with detailed logging
./04-execute.sh --local --debug  # Recommended: always use --debug for troubleshooting

# Use installed bonsol with detailed logging
./04-execute.sh --debug
```

## ⚙️ Configuration

### Environment Variables
```bash
# Build Configuration
RISC0_DEV_MODE=1                    # Enable dev mode
RUST_LOG="debug,risc0_zkvm=debug"   # Configure logging
RUST_BACKTRACE=1                    # Enable backtraces

# AWS Configuration
AWS_ACCESS_KEY_ID=your_access_key
AWS_SECRET_ACCESS_KEY=your_secret_key
AWS_REGION=us-east-1
BUCKET=8bitoracle
# Optional: S3_ENDPOINT=https://custom.s3.endpoint
```

### Script Flags
| Flag | Description |
|:-----|:------------|
| `--local` | Use locally built binaries instead of installed bonsol |
| `--debug` | Enable verbose logging (recommended for troubleshooting) |
| `--rebuild-bonsol` | Rebuild entire workspace |

## 🎯 Callback Setup

### Program ID Verification
Unlike the sample callback program (`exay1T7QqsJPNcwzMiWubR6vZnqrgM16jZRraHgqBGG`) which is a static reference example, this I Ching callback program is meant to be deployed individually by users. Each deployment requires its own unique program ID because:

- On Solana, program IDs are derived from the deployment keypair
- Two users cannot deploy to the same program ID
- Each user needs their own keypair and corresponding program ID

This is why we need a manual verification step:
1. The program ID in the source code must match your keypair's program ID
2. This can't be automated at runtime since it's part of the compiled code
3. If they don't match, deployment will fail because Solana verifies program ownership

### Storage Model
This is a sample implementation with simplified storage:
- Each I Ching reading uses a new PDA (Program Derived Address) storage account
- Each storage account holds exactly one hexagram reading
- No historical readings are maintained
- Each new reading creates a new storage account
- This is intentionally simplified for demonstration purposes

### Setup Steps
1. Get and verify the program ID:
```bash
# Get the program ID from your keypair
solana-keygen pubkey onchain/8bitoracle-iching-callback/scripts/program-keypair.json | cat

# Update the program ID in lib.rs to match your keypair
code onchain/8bitoracle-iching-callback/src/lib.rs
# Find and update the line:
# solana_program::declare_id!("your_program_id_here");
```

2. Deploy the callback program:
```bash
cd onchain/8bitoracle-iching-callback
cargo build-sbf
solana program deploy target/deploy/bitoracle_iching_callback.so
```

3. Verify deployment:
```bash
solana program show --programs
```

> ⚠️ **Important**: The program ID in `lib.rs` MUST match your keypair's program ID before building and deploying. This is a manual step that must be completed for successful deployment.

## 🌐 Validator Setup

1. Start the local validator:
```bash
bin/validator.sh
```
This script:
- Builds all onchain programs
- Starts a local Solana validator
- Must be running before deploying or executing programs

2. Verify validator is running:
```bash
solana config get
solana cluster-version
```

## ⚠️ Important Notes

> 🏠 **Working Directory**
- Always run scripts from project root
- Scripts handle directory changes automatically
- No manual directory navigation needed

> 🔐 **Environment Variables**
- Use `source` to load variables: `. images/8bitoracle-iching/.env`
- Direct execution won't persist variables
- Required for S3 deployment

> 🛠️ **Build Process**
- Single script handles all build steps
- Includes Rust and ZK program compilation
- No manual build steps needed

> ☁️ **AWS Setup**
- S3 bucket must exist and be accessible
- Not required with `--local` flag

> 🔍 **Debugging**
- Using `--debug` is recommended for all operations
- Provides detailed logging for troubleshooting
- Shows important execution details and progress
- Helps identify any issues quickly

> 💾 **Storage**
- Results stored in callback program PDA
- Storage initialized during input generation
- View results: `solana account <HEXAGRAM_PDA>` 