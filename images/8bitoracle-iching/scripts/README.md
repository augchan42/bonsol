# 🎲 I Ching Oracle Scripts

> 💫 This directory contains scripts for building, deploying, and executing the I Ching Oracle Zero-Knowledge program.

## 📋 Prerequisites

1. `bonsol` installed and in your PATH
2. Rust and Cargo ([rustup.rs](https://rustup.rs))
3. AWS credentials (if not using local deployment)
4. Solana CLI tools
5. Callback program deployed (see [Callback Setup](#-callback-setup))

> ℹ️ **Note:** Run all commands from the project root directory (`~/forked-projects/bonsol` or equivalent)

## 🚀 Quick Start

1. Make scripts executable:
```bash
chmod +x images/8bitoracle-iching/scripts/*.sh
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
# Build everything
images/8bitoracle-iching/scripts/01-build.sh

# Deploy using locally built binaries (recommended with debug)
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
Handles the complete build process in three stages:

#### Onchain Programs
```bash
# Build Solana programs
cargo build-sbf
```
- Main Bonsol program
- Example program
- I Ching callback program

#### Bonsol Workspace (Optional)
```bash
# With --rebuild-bonsol flag
cargo build --workspace
```

#### I Ching Program
```bash
# Main program build
cargo build
```

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

## 🔐 Callback Setup

1. Deploy the callback program:
```bash
cd onchain/8bitoracle-iching-callback
cargo build-sbf
solana program deploy target/deploy/bitoracle_iching_callback.so
```

2. Verify deployment:
```bash
solana program show --programs
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