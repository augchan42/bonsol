#!/bin/bash

# Exit on error
set -e

# Function to check if Docker is available and running
check_docker() {
    echo "Checking Docker status..."

    # Check if docker command exists
    if ! command -v docker &>/dev/null; then
        echo "Error: Docker is not installed. Please install Docker first:"
        echo "Ubuntu: sudo apt-get install docker.io"
        echo "Or follow installation guide at: https://docs.docker.com/engine/install/"
        exit 1
    fi

    # Check if Docker service is running
    if ! docker info &>/dev/null; then
        echo "Docker daemon is not running. Attempting to start Docker..."

        # Try to start Docker service
        if command -v systemctl &>/dev/null; then
            echo "Starting Docker with systemctl..."
            sudo systemctl start docker || {
                echo "Failed to start Docker with systemctl"
                echo "Please start Docker manually:"
                echo "sudo systemctl start docker"
                exit 1
            }
        elif command -v service &>/dev/null; then
            echo "Starting Docker with service command..."
            sudo service docker start || {
                echo "Failed to start Docker with service command"
                echo "Please start Docker manually:"
                echo "sudo service docker start"
                exit 1
            }
        else
            echo "Could not start Docker automatically."
            echo "Please start Docker manually using your system's service manager."
            exit 1
        fi

        # Wait for Docker to be ready
        echo "Waiting for Docker to be ready..."
        for i in {1..30}; do
            if docker info &>/dev/null; then
                echo "Docker is now running!"
                break
            fi
            if [ $i -eq 30 ]; then
                echo "Timeout waiting for Docker to start"
                exit 1
            fi
            sleep 1
        done
    fi

    # Verify user has permission to use Docker
    if ! docker ps &>/dev/null; then
        echo "Error: Current user doesn't have permission to use Docker."
        echo "Try adding your user to the docker group:"
        echo "sudo usermod -aG docker $USER"
        echo "Then log out and back in, or run:"
        echo "newgrp docker"
        exit 1
    fi

    echo "Docker is running and accessible ✓"
}

# Function to validate input format
validate_input() {
    echo "Validating input.json format..."
    INPUT_FILE="images/8bitoracle-iching/input.json"

    if [ ! -f "$INPUT_FILE" ]; then
        echo "Error: input.json not found at $INPUT_FILE"
        echo "Please run 03-generate-input-with-callback.sh first"
        exit 1
    fi

    # Check if input.json is valid JSON
    if ! jq '.' "$INPUT_FILE" >/dev/null 2>&1; then
        echo "Error: input.json is not valid JSON"
        exit 1
    fi

    # Validate required fields
    REQUIRED_FIELDS=("imageId" "executionId" "executionConfig" "inputs")
    for field in "${REQUIRED_FIELDS[@]}"; do
        if ! jq -e ".$field" "$INPUT_FILE" >/dev/null 2>&1; then
            echo "Error: Missing required field '$field' in input.json"
            exit 1
        fi
    done

    # Validate input format
    if ! jq -e '.inputs[0].inputType == "PublicData"' "$INPUT_FILE" >/dev/null 2>&1; then
        echo "Error: First input must be of type 'PublicData'"
        exit 1
    fi

    # Validate input data format (should be hex)
    INPUT_DATA=$(jq -r '.inputs[0].data' "$INPUT_FILE")
    if [[ ! "$INPUT_DATA" =~ ^0x[0-9a-fA-F]+$ ]]; then
        echo "Error: Input data must be hex format starting with '0x'"
        echo "Found: $INPUT_DATA"
        exit 1
    fi

    echo "✓ Input validation passed"
}

# Check Docker before proceeding
check_docker

# Validate input.json
validate_input

# Parse command line arguments
USE_LOCAL=false
DEBUG=true # Always enable debug mode for better error messages
REBUILD_BONSOL=false
while [[ "$#" -gt 0 ]]; do
    case $1 in
    --local)
        USE_LOCAL=true
        shift
        ;;
    --debug)
        DEBUG=true
        shift
        ;;
    --rebuild-bonsol)
        REBUILD_BONSOL=true
        shift
        ;;
    *)
        echo "Unknown parameter: $1"
        exit 1
        ;;
    esac
done

# Source environment variables
ENV_FILE="$(dirname "$0")/../.env"
if [ -f "$ENV_FILE" ]; then
    echo "Loading environment variables from $ENV_FILE"
    set -a # automatically export all variables
    source "$ENV_FILE"
    set +a
else
    echo "Warning: .env file not found at $ENV_FILE"
    exit 1
fi

# Enable RISC0 debug mode and logging
export RISC0_DEV_MODE=1
export RUST_LOG="debug,risc0_zkvm=debug"
export RUST_BACKTRACE=1

echo "Build Configuration:"
echo "  RISC0_DEV_MODE: $RISC0_DEV_MODE"
echo "  RUST_LOG: $RUST_LOG"
echo "  RUST_BACKTRACE: $RUST_BACKTRACE"
echo "  Debug mode: $DEBUG"

# Store original directory and find project root
ORIGINAL_DIR=$(pwd)
PROJECT_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
echo "Project root directory: $PROJECT_ROOT"

# Step 1: Always build onchain programs
echo "Step 1: Building onchain programs..."
if [ "$DEBUG" = true ]; then
    # First build the Solana smart contract program with dev mode and logging
    echo "Debug: Building Solana program with dev mode..."
    cd "$PROJECT_ROOT/onchain/bonsol"
    RUST_LOG="debug,solana_program::log=debug,bonsol=debug" \
        RUST_BACKTRACE=1 \
        RISC0_DEV_MODE=1 \
        cargo build-sbf --verbose
    
    # Build the callback example program
    echo "Debug: Building callback example program..."
    cd "$PROJECT_ROOT/onchain/example-program-on-bonsol"
    RUST_LOG="debug,solana_program::log=debug" \
        RUST_BACKTRACE=1 \
        cargo build-sbf --verbose
    
    # Build the 8BitOracle I Ching callback program
    echo "Debug: Building 8BitOracle I Ching callback program..."
    cd "$PROJECT_ROOT/onchain/8bitoracle-iching-callback"
    RUST_LOG="debug,solana_program::log=debug" \
        RUST_BACKTRACE=1 \
        cargo build-sbf --verbose
else
    cd "$PROJECT_ROOT/onchain/bonsol"
    cargo build-sbf
    cd "$PROJECT_ROOT/onchain/example-program-on-bonsol"
    cargo build-sbf
    cd "$PROJECT_ROOT/onchain/8bitoracle-iching-callback"
    cargo build-sbf
fi
echo "Onchain programs build complete"
echo

# Step 2: Optionally rebuild entire workspace
if [ "$REBUILD_BONSOL" = true ]; then
    echo "Step 2: Rebuilding entire bonsol workspace..."
    cd "$PROJECT_ROOT"
    if [ "$DEBUG" = true ]; then
        echo "Debug: Building workspace in directory: $(pwd)"
        echo "Debug: Cleaning previous build..."
        cargo clean

        echo "Debug: Building workspace with dev mode..."
        RUST_LOG="debug,bonsol=debug,risc0_runner=debug,solana_program::log=debug" \
            RUST_BACKTRACE=1 \
            RISC0_DEV_MODE=1 \
            cargo build --verbose --workspace
    else
        cargo build --workspace
    fi
    echo "Bonsol workspace rebuild complete"
    echo
fi

# Step 3: Build I Ching program
echo "Step 3: Building I Ching program..."
cd "$PROJECT_ROOT/images/8bitoracle-iching"
if [ "$DEBUG" = true ]; then
    echo "Debug: Build Configuration:"
    echo "  Current directory: $(pwd)"
    echo "  Original directory: $ORIGINAL_DIR"
    echo "  Project root: $PROJECT_ROOT"
    echo "  Environment:"
    echo "    RUST_LOG=$RUST_LOG"
    echo "    RUST_BACKTRACE=$RUST_BACKTRACE"
    echo "    RISC0_DEV_MODE=$RISC0_DEV_MODE"
    echo "    BONSOL_HOME=$BONSOL_HOME"
fi

echo "Building I Ching program..."
if [ "$DEBUG" = true ]; then
    echo "Debug: Running cargo with verbose output and features: $CARGO_FLAGS"
    cargo build --verbose $CARGO_FLAGS
else
    cargo build $CARGO_FLAGS
fi

# Run bonsol build to generate manifest.json
# Architecture Notes:
# This project uses a split architecture:
# 1. ZK Program (RISC0)
#    - Runs computation off-chain and generates proofs
#    - Uses STARK/SNARK system (exact performance implications unclear)
#    - Deployed to S3 for prover nodes to access
# 2. Solana Callback Program
#    - On-chain program that receives and processes proofs
#    - Handles storage of results
# Note: The tradeoffs and limitations of this approach (especially around
# ZK circuit complexity and proving time) would need careful benchmarking
# to fully understand.

# Building ZK program and generating manifest.json...
echo "Running bonsol build to generate manifest.json..."
bonsol build --zk-program-path .

# Return to original directory
cd "$ORIGINAL_DIR"
echo "Build process complete!"

echo
echo "Build complete! You can now run 02-deploy.sh to deploy the program."
echo
if [ "$DEBUG" = true ]; then
    echo "Debug: Final Status:"
    echo "  Rust program built in: $(dirname "$0")/../target"
    echo "  ZK program built in: images/8bitoracle-iching"
    echo "  RISC0_DEV_MODE: $RISC0_DEV_MODE"
    if [ -f "images/8bitoracle-iching/manifest.json" ]; then
        echo "  Updated manifest contents:"
        cat images/8bitoracle-iching/manifest.json
    fi
    echo "  Environment variables for next steps:"
    echo "    BONSOL_S3_ENDPOINT=$BONSOL_S3_ENDPOINT"
    echo "    BONSOL_S3_BUCKET=$BONSOL_S3_BUCKET"
    echo "    RUST_LOG=$RUST_LOG"
fi

echo "Note: You can use the following flags:"
echo "      --local           Use local bonsol build from target/debug/"
echo "      --debug           Enable detailed logging and dev mode"
echo "      --rebuild-bonsol  Rebuild the bonsol node software"
