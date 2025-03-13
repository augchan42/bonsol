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

# Store original directory
ORIGINAL_DIR=$(pwd)

# Step 0: Rebuild bonsol if requested
if [ "$REBUILD_BONSOL" = true ]; then
    echo "Step 0: Rebuilding bonsol node software..."
    if [ "$DEBUG" = true ]; then
        echo "Debug: Building bonsol in directory: $ORIGINAL_DIR"
        echo "Debug: Cleaning previous build..."
        cargo clean

        # First build the Solana smart contract program with dev mode and logging
        echo "Debug: Building Solana program with dev mode..."
        cd onchain/bonsol
        RUST_LOG="debug,solana_program::log=debug,bonsol=debug" \
            RUST_BACKTRACE=1 \
            RISC0_DEV_MODE=1 \
            cargo build-sbf --verbose
        cd ../..

        # Build the callback example program
        echo "Debug: Building callback example program..."
        cd onchain/example-program-on-bonsol
        RUST_LOG="debug,solana_program::log=debug" \
            RUST_BACKTRACE=1 \
            cargo build-sbf --verbose
        cd ../..

        # Build the 8BitOracle I Ching callback program
        echo "Debug: Building 8BitOracle I Ching callback program..."
        cd onchain/8bitoracle-iching-callback
        RUST_LOG="debug,solana_program::log=debug" \
            RUST_BACKTRACE=1 \
            cargo build-sbf --verbose
        cd ../..

        echo "Debug: Building rest of bonsol with dev mode..."
        RUST_LOG="debug,bonsol=debug,risc0_runner=debug,solana_program::log=debug" \
            RUST_BACKTRACE=1 \
            RISC0_DEV_MODE=1 \
            cargo build --verbose
    else
        cargo build
    fi
    echo "Bonsol rebuild complete"
    echo

    # Build the 8BitOracle I Ching callback program
    echo "Building 8BitOracle I Ching callback program..."
    cd onchain/8bitoracle-iching-callback
    cargo build-sbf
    cd ../..
fi

echo "Step 1: Building Rust program..."
if [ "$DEBUG" = true ]; then
    echo "Debug: Build Configuration:"
    echo "  Current directory: $(pwd)"
    echo "  Original directory: $ORIGINAL_DIR"
    echo "  Environment:"
    echo "    RUST_LOG=$RUST_LOG"
    echo "    RUST_BACKTRACE=$RUST_BACKTRACE"
    echo "    RISC0_DEV_MODE=$RISC0_DEV_MODE"
    echo "    BONSOL_HOME=$BONSOL_HOME"
fi

echo "Changing to I Ching program directory..."
cd "$(dirname "$0")/.." # Change to iching program directory (up one level from scripts)
if [ "$DEBUG" = true ]; then
    echo "Debug: Changed to directory: $(pwd)"
    echo "Debug: Contents of current directory:"
    ls -la
    echo "Debug: Cargo.toml contents:"
    cat Cargo.toml
fi

echo "Running cargo build..."
if [ "$DEBUG" = true ]; then
    echo "Debug: Running cargo with verbose output and features: $CARGO_FLAGS"
    cargo build --verbose $CARGO_FLAGS
else
    cargo build
fi

echo
echo "Step 2: Building ZK program..."
echo "Changing back to project root..."
cd "$ORIGINAL_DIR" # Return to original directory for bonsol build
if [ "$DEBUG" = true ]; then
    echo "Debug: Changed back to directory: $(pwd)"
fi

# Determine which bonsol to use
if [ "$USE_LOCAL" = true ]; then
    if [ -f "${BONSOL_HOME}/target/debug/bonsol" ]; then
        BONSOL_CMD="${BONSOL_HOME}/target/debug/bonsol"
        echo "Using local bonsol build: $BONSOL_CMD"
        if [ "$DEBUG" = true ]; then
            echo "Debug: Bonsol binary details:"
            ls -l "$BONSOL_CMD"
            echo "Debug: Bonsol binary last modified:"
            stat "$BONSOL_CMD"
        fi
    else
        echo "Error: Local bonsol build not found at ${BONSOL_HOME}/target/debug/bonsol"
        echo "Please build bonsol locally first using 'cargo build'"
        exit 1
    fi
else
    BONSOL_CMD="bonsol"
    echo "Using installed bonsol from PATH"
    if [ "$DEBUG" = true ]; then
        echo "Debug: Bonsol path:"
        which bonsol
    fi
fi

if [ "$DEBUG" = true ]; then
    echo "Debug: ZK Program Configuration:"
    echo "  Program path: images/8bitoracle-iching"
    echo "  Bonsol command: $BONSOL_CMD"
    echo "  RISC0_DEV_MODE: $RISC0_DEV_MODE"
    if [ -f "images/8bitoracle-iching/manifest.json" ]; then
        echo "Debug: Current manifest contents:"
        cat images/8bitoracle-iching/manifest.json
    fi
fi

echo "Running bonsol build..."
if [ "$DEBUG" = true ]; then
    echo "Debug: Running bonsol build command: $BONSOL_CMD build --zk-program-path images/8bitoracle-iching"
    # Clean the ZK program build to ensure rebuild with dev mode
    rm -rf images/8bitoracle-iching/target
    "$BONSOL_CMD" build --zk-program-path images/8bitoracle-iching
    echo "Debug: Build command completed"
else
    "$BONSOL_CMD" build --zk-program-path images/8bitoracle-iching
fi

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
