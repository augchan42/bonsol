# I Ching Oracle Scripts

This directory contains scripts for building, deploying, and executing the I Ching Oracle ZK program.

## Prerequisites

1. Install `bonsol` and ensure it's in your PATH
2. Install Rust and Cargo (https://rustup.rs)
3. Configure your AWS credentials in environment variables
4. Run all commands from the project root directory (`~/forked-projects/bonsol` or equivalent)
5. Deploy the callback program (see Callback Program Setup below)

## Build Process

The build script (`01-build.sh`) handles a three-step build process:

### 1. Onchain Programs Build
- Builds all Solana smart contract programs using `cargo build-sbf`:
  - Main Bonsol program (`onchain/bonsol`)
  - Example program (`onchain/example-program-on-bonsol`)
  - I Ching callback program (`onchain/8bitoracle-iching-callback`)
- Debug mode enables verbose output and extended logging

### 2. Optional Bonsol Workspace Rebuild
- Triggered with `--rebuild-bonsol` flag
- Rebuilds entire Bonsol workspace from project root
- Uses `cargo build --workspace`
- Debug mode includes additional logging and dev features

### 3. I Ching Program Build
- Builds the main I Ching program in `images/8bitoracle-iching`
- Uses standard `cargo build` with configurable debug flags

### Build Configuration
```bash
# Environment Variables
RISC0_DEV_MODE=1        # Enable RISC0 development mode
RUST_LOG="debug,risc0_zkvm=debug"  # Configure logging
RUST_BACKTRACE=1        # Enable full backtraces
```

### Build Flags
- `--local`: Deploy locally instead of to S3
- `--debug`: Enable verbose output and extended logging
- `--rebuild-bonsol`: Trigger full workspace rebuild

### Prerequisites Validation
The build script automatically checks:
- Docker availability and permissions
- Input file validation (`input.json`)
- Environment file presence (`.env`)
- Required dependencies

## Script Order and Purpose

1. `01-build.sh`: Builds both the Rust program and the ZK program
   - First compiles the I Ching Rust code in `images/8bitoracle-iching`
   - Then builds the ZK program using `bonsol build`
   - Handles directory changes automatically

2. `02-deploy.sh`: Deploys the program to S3 or locally
   - Requires successful completion of the build step
   - Uses AWS credentials to upload to S3 (if not local)
   - Flags:
     * `--local`: Deploy locally instead of to S3
     * `--debug`: Enable debug output

3. `03-generate-input-with-callback.sh`: Generates input for program execution with callback configuration
   - Uses the Image ID from the deploy step
   - Sets up the callback program PDA for storing hexagram results
   - Initializes storage account if needed

4. `04-execute.sh`: Executes the program and displays results
   - Uses the generated input to run the program
   - Generates and stores the I Ching hexagram
   - Flags:
     * `--debug`: Enable detailed debug output
     * `--local`: Use local deployment

## Callback Program Setup

Before running the scripts, deploy the callback program:

```bash
# Build and deploy the callback program
cd onchain/8bitoracle-iching-callback
cargo build-sbf
solana program deploy target/deploy/bitoracle_iching_callback.so

# Note the program ID for verification
solana program show --programs
```

## Environment Setup

Create a `.env` file in the `images/8bitoracle-iching` directory:

```env
# AWS Credentials (not needed for --local)
AWS_ACCESS_KEY_ID=your_access_key_here
AWS_SECRET_ACCESS_KEY=your_secret_key_here
AWS_REGION=us-east-1

# S3 Configuration (not needed for --local)
BUCKET=8bitoracle
# Optional: S3_ENDPOINT=https://custom.s3.endpoint
```

## Usage

From the project root directory:

```bash
# 1. Make scripts executable (only needed once)
chmod +x images/8bitoracle-iching/scripts/*.sh

# 2. Source your environment variables (if using S3)
source images/8bitoracle-iching/.env

# 3. Build the program (this handles both Rust and ZK builds)
images/8bitoracle-iching/scripts/01-build.sh

# 4. Deploy (choose one):
# For local deployment with debug output:
images/8bitoracle-iching/scripts/02-deploy.sh --local --debug

# For S3 deployment:
images/8bitoracle-iching/scripts/02-deploy.sh

# 5. Generate input with callback configuration
images/8bitoracle-iching/scripts/03-generate-input-with-callback.sh

# 6. Execute the program (choose one):
# For local execution with debug output:
images/8bitoracle-iching/scripts/04-execute.sh --local --debug

# For normal execution:
images/8bitoracle-iching/scripts/04-execute.sh
```

## Important Notes

1. **Working Directory**: 
   - Run all scripts from the project root directory
   - The build script will automatically change to the correct directories for each step
   - You don't need to manually change directories

2. **Environment Variables**: 
   - Use `source` or `.` to load environment variables: `. images/8bitoracle-iching/.env`
   - Direct execution (`./env.sh`) won't persist variables in your shell
   - Environment variables must be loaded before running deploy (if not using --local)

3. **Build Process**:
   - The build script (`01-build.sh`) handles both Rust compilation and ZK program building
   - Rust compilation happens in the I Ching program directory
   - ZK program building happens from the project root
   - You don't need to run any manual build steps

4. **AWS Configuration**: 
   - S3 bucket must exist and be accessible with your credentials
   - Not needed when using `--local` flag

5. **Debug Output**:
   - Use `--debug` flag with deploy and execute scripts for detailed logging
   - Helpful for troubleshooting issues with program execution or callbacks

6. **Callback Storage**:
   - The program stores hexagram results in a PDA owned by the callback program
   - Storage account is automatically initialized during input generation
   - Use `solana account <HEXAGRAM_PDA>` to view stored results

Each script will guide you to the next step in the process. 