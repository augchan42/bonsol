#!/bin/bash

# Exit on error
set -e

# Function to check if a command exists
check_dependency() {
    if ! command -v "$1" &>/dev/null; then
        echo "Error: $1 is required but not installed."
        echo "Please install $1 first"
        exit 1
    fi
}

# Check required dependencies
echo "Checking dependencies..."
REQUIRED_DEPS=("jq" "bc" "solana" "solana-keygen")
for dep in "${REQUIRED_DEPS[@]}"; do
    check_dependency "$dep"
done
echo "✓ All dependencies found"

# Parse command line arguments
USE_LOCAL=false
DEBUG=false
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
    *)
        echo "Unknown parameter: $1"
        exit 1
        ;;
    esac
done

# Store original directory
ORIGINAL_DIR=$(pwd)

# Get project root directory (3 levels up from script location)
PROJECT_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
echo "Project root directory: $PROJECT_ROOT"

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

echo "----------------------------------------"
echo "Starting image ID extraction process..."

# Extract image ID from input.json - use absolute path
INPUT_PATH="$PROJECT_ROOT/images/8bitoracle-iching/input.json"
echo "Looking for input file at: $INPUT_PATH"

if [ ! -f "$INPUT_PATH" ]; then
    echo "Error: Input file not found at $INPUT_PATH"
    echo "Please run 03-generate-input.sh first to create the input file."
    exit 1
fi

echo "Found input file. Contents:"
echo "----------------------------------------"
cat "$INPUT_PATH" | jq '.'
echo "----------------------------------------"

echo "Extracting imageId from input file..."
export BONSOL_IMAGE_ID=$(jq -r '.imageId' "$INPUT_PATH")
if [ -z "$BONSOL_IMAGE_ID" ] || [ "$BONSOL_IMAGE_ID" = "null" ]; then
    echo "Error: Could not extract imageId from input.json"
    exit 1
fi

echo "Successfully extracted image ID: $BONSOL_IMAGE_ID"
echo "----------------------------------------"

# Always enable RISC0_DEV_MODE for development/testing
export RISC0_DEV_MODE=1
echo "RISC0_DEV_MODE enabled for development/testing"

# Enable debug logging if --debug flag is passed
if [ "$DEBUG" = true ]; then
    echo "Debug mode enabled"
    echo "Setting up logging configuration..."
    export RUST_LOG="risc0_zkvm=debug,bonsol_prover::input_resolver=debug,solana_program::log=debug,bonsol=info,solana_program=debug,risc0_zkvm::guest=debug,solana_runtime::message_processor=trace,solana_program_runtime=debug,solana_runtime=debug"
    export RUST_BACKTRACE=full
    echo "RUST_LOG set to: $RUST_LOG"
    echo "Full backtraces enabled"
fi

# Set BONSOL_S3_ENDPOINT with base URL only (no bucket)
if [ -n "$S3_ENDPOINT" ]; then
    echo "Configuring S3 settings..."
    # Remove any existing protocol and trailing slash
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT#https://}
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT_CLEAN#http://}
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT_CLEAN%/}

    # Add https:// but NOT the bucket
    export BONSOL_S3_ENDPOINT="https://$S3_ENDPOINT_CLEAN"
    export BONSOL_S3_BUCKET="${BUCKET:-8bitoracle}"
    export BONSOL_S3_PATH_FORMAT="iching-{image_id}"

    echo "S3 Configuration:"
    echo "  Base URL: $BONSOL_S3_ENDPOINT"
    echo "  Bucket: $BONSOL_S3_BUCKET"
    echo "  Path format: $BONSOL_S3_PATH_FORMAT"
    echo "  Image ID: $BONSOL_IMAGE_ID"

    FINAL_URL="$BONSOL_S3_ENDPOINT/$BONSOL_S3_BUCKET/iching-$BONSOL_IMAGE_ID"
    echo "Final S3 URL will be: $FINAL_URL"
    echo "----------------------------------------"
fi

# Determine which bonsol to use
if [ "$USE_LOCAL" = true ]; then
    if [ -f "${BONSOL_HOME}/target/debug/bonsol" ]; then
        BONSOL_CMD="${BONSOL_HOME}/target/debug/bonsol"
        echo "Using local bonsol build: $BONSOL_CMD"
    else
        echo "Error: Local bonsol build not found at ${BONSOL_HOME}/target/debug/bonsol"
        echo "Please build bonsol locally first using 'cargo build'"
        exit 1
    fi
else
    BONSOL_CMD="bonsol"
    echo "Using installed bonsol from PATH"
fi

echo "----------------------------------------"
echo "Environment variables that will be used:"
echo "BONSOL_IMAGE_ID=$BONSOL_IMAGE_ID"
echo "BONSOL_S3_ENDPOINT=$BONSOL_S3_ENDPOINT"
echo "BONSOL_S3_BUCKET=$BONSOL_S3_BUCKET"
echo "BONSOL_S3_PATH_FORMAT=$BONSOL_S3_PATH_FORMAT"
echo "RUST_LOG=$RUST_LOG"
echo "RUST_BACKTRACE=$RUST_BACKTRACE"
echo "RISC0_DEV_MODE=$RISC0_DEV_MODE"
echo "----------------------------------------"

# Get the test execution keypair path
EXECUTION_KEYPAIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/test-execution-keypair.json"
if [ ! -f "$EXECUTION_KEYPAIR" ]; then
    echo "Error: Test execution keypair not found at $EXECUTION_KEYPAIR"
    exit 1
fi
echo "Using test execution keypair: $EXECUTION_KEYPAIR"

# Get the test payer keypair path
PAYER_KEYPAIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/test-payer-keypair.json"
if [ ! -f "$PAYER_KEYPAIR" ]; then
    echo "Error: Test payer keypair not found at $PAYER_KEYPAIR"
    exit 1
fi
echo "Using test payer keypair: $PAYER_KEYPAIR"

# Store the original Solana config
echo "Storing original Solana configuration..."
ORIGINAL_KEYPAIR=$(solana config get | grep "Keypair Path" | awk '{print $3}')
if [ "$ORIGINAL_KEYPAIR" = "$PAYER_KEYPAIR" ]; then
    # If original keypair is already the test payer, use the default location
    ORIGINAL_KEYPAIR="$HOME/.config/solana/id.json"
fi
echo "Original keypair path: $ORIGINAL_KEYPAIR"

# Set the payer keypair as default and verify
if ! solana config set --keypair "$PAYER_KEYPAIR"; then
    echo "Error: Failed to set Solana config"
    exit 1
fi

# Verify config was set correctly
CURRENT_KEYPAIR=$(solana config get | grep "Keypair Path" | awk '{print $3}')
if [ "$CURRENT_KEYPAIR" != "$PAYER_KEYPAIR" ]; then
    echo "Error: Solana config not set correctly"
    echo "Expected: $PAYER_KEYPAIR"
    echo "Got: $CURRENT_KEYPAIR"
    exit 1
fi
echo "Set default Solana keypair to: $PAYER_KEYPAIR"

# Get the public keys for both accounts
REQUESTER_PUBKEY=$(solana-keygen pubkey "$EXECUTION_KEYPAIR")
PAYER_PUBKEY=$(solana-keygen pubkey "$PAYER_KEYPAIR")

if [ -z "$REQUESTER_PUBKEY" ] || [ -z "$PAYER_PUBKEY" ]; then
    echo "Error: Could not get public keys from keypairs"
    exit 1
fi
echo "Requester account public key: $REQUESTER_PUBKEY"
echo "Payer account public key: $PAYER_PUBKEY"

# Check payer balance and handle airdrop if needed
BALANCE=$(solana balance "$PAYER_PUBKEY" | awk '{print $1}')
echo "Current payer balance: $BALANCE SOL"

if (($(echo "$BALANCE < 1" | bc -l))); then
    echo "Payer balance too low for execution"
    
    # Get current cluster
    CLUSTER=$(solana config get | grep "RPC URL" | awk '{print $3}')
    if [[ "$CLUSTER" == *"mainnet"* ]]; then
        echo "Error: Insufficient funds on mainnet. Please fund payer account manually."
        exit 1
    else
        echo "Attempting to airdrop 10 SOL to payer account..."
        # Try airdrop up to 3 times
        for i in {1..3}; do
            if solana airdrop 10 "$PAYER_PUBKEY"; then
                echo "Airdrop successful!"
                break
            else
                if [ $i -eq 3 ]; then
                    echo "Error: Airdrop failed after 3 attempts. Please fund account manually or try again later."
                    exit 1
                fi
                echo "Airdrop attempt $i failed. Retrying..."
                sleep 2
            fi
        done
        
        # Verify new balance
        NEW_BALANCE=$(solana balance "$PAYER_PUBKEY" | awk '{print $1}')
        echo "New payer balance: $NEW_BALANCE SOL"
        
        if (($(echo "$NEW_BALANCE < 1" | bc -l))); then
            echo "Error: Payer balance still too low after airdrop. Please fund account manually."
            exit 1
        fi
    fi
fi

echo "Payer balance check passed ✓"

# Also check execution account balance
EXEC_BALANCE=$(solana balance "$REQUESTER_PUBKEY" | awk '{print $1}')
echo "Current execution account balance: $EXEC_BALANCE SOL"

if (($(echo "$EXEC_BALANCE < 0.1" | bc -l))); then
    echo "Execution account balance low, transferring 0.1 SOL from payer..."
    if ! solana transfer --allow-unfunded-recipient "$REQUESTER_PUBKEY" 0.1 --keypair "$PAYER_KEYPAIR"; then
        echo "Error: Failed to transfer SOL to execution account"
        exit 1
    fi
    echo "Transfer successful"
fi

echo "Execution account balance check passed ✓"
echo "----------------------------------------"

echo "----------------------------------------"
echo "Verifying input.json before execution..."
if ! jq '.' "$INPUT_PATH" >/dev/null 2>&1; then
    echo "Error: input.json is not valid JSON"
    exit 1
fi

# Verify input format
echo "Checking input format..."
if ! jq -e '.inputs[0].inputType == "PublicData"' "$INPUT_PATH" >/dev/null 2>&1; then
    echo "Error: First input must be of type 'PublicData'"
    exit 1
fi

# Verify input data format
INPUT_DATA=$(jq -r '.inputs[0].data' "$INPUT_PATH")
if [[ ! "$INPUT_DATA" =~ ^0x[0-9a-fA-F]+$ ]]; then
    echo "Error: Input data must be hex format starting with '0x'"
    echo "Found: $INPUT_DATA"
    exit 1
fi

echo "Input validation passed ✓"
echo "----------------------------------------"

echo "----------------------------------------"
echo "Verifying program deployment..."

# Get the callback program ID from input.json
CALLBACK_PROGRAM_ID=$(jq -r '.callbackConfig.programId' "$INPUT_PATH")
echo "Callback program ID from input.json: $CALLBACK_PROGRAM_ID"

# Check if the callback program exists
echo "Checking callback program deployment..."
if ! solana program show "$CALLBACK_PROGRAM_ID" &>/dev/null; then
    echo "Error: Callback program not found at $CALLBACK_PROGRAM_ID"
    echo "Please ensure the program is deployed first"
    exit 1
fi
echo "✓ Callback program found"

echo "----------------------------------------"

# Calculate space for HexagramData
# Space calculation breakdown:
# - 8 bytes for Anchor discriminator
# - 6 bytes for lines [u8; 6]
# - 1024 bytes for ascii_art String (max size)
# - 8 bytes for timestamp i64
# - 1 byte for is_initialized bool
HEXAGRAM_SPACE=$((\
    8 + \
    6 + \
    1024 + \
    8 + \
    1))

echo "Verifying hexagram storage account configuration..."
echo "- Space required: $HEXAGRAM_SPACE bytes"

# Get the timestamp from input.json
TIMESTAMP=$(jq -r '.timestamp' "$INPUT_PATH")
echo "- Timestamp: $TIMESTAMP"

# Note: We don't need to create PDAs manually - they are created on-chain
# during the first instruction that uses them. Just verify our configuration:
echo "Verifying account configuration..."
echo "Note: Bonsol prepends these accounts: requester(0), execution(1), callback_program(2), prover(3)"
echo "Extra accounts start at index 4:"

# Get and validate the account configuration
HEXAGRAM_PDA=$(jq -r '.callbackConfig.extraAccounts[0].pubkey' "$INPUT_PATH")
SYSTEM_PROGRAM=$(jq -r '.callbackConfig.extraAccounts[1].pubkey' "$INPUT_PATH")
DEPLOYMENT_PDA=$(jq -r '.callbackConfig.extraAccounts[2].pubkey' "$INPUT_PATH")

echo "- Hexagram Account (account[4]): $HEXAGRAM_PDA"
if [ "$SYSTEM_PROGRAM" != "11111111111111111111111111111111" ]; then
    echo "Error: System Program account (account[5]) must be 11111111111111111111111111111111"
    echo "Got: $SYSTEM_PROGRAM"
    exit 1
fi
echo "- System Program (account[5]): $SYSTEM_PROGRAM"
echo "- Deployment Account (account[6]): $DEPLOYMENT_PDA"

echo "Account configuration verified ✓"
echo "----------------------------------------"

echo "Executing I Ching program..."
if [ "$DEBUG" = true ]; then
    echo "Running with debug configuration:"
    echo "Command: $BONSOL_CMD execute -f \"$INPUT_PATH\" --wait"
    echo "Input file contents:"
    cat "$INPUT_PATH" | jq '.'
    echo "----------------------------------------"

    # Run with debug output and trace-level logging
    RUST_LOG="$RUST_LOG,solana_runtime=trace" \
        BONSOL_REQUESTER_KEYPAIR="$EXECUTION_KEYPAIR" \
        BONSOL_PAYER_KEYPAIR="$PAYER_KEYPAIR" \
        "$BONSOL_CMD" execute -f "$INPUT_PATH" \
        --wait || {
        echo "Error: Execution failed!"
        echo "Please check the error messages above for details."
        exit 1
    }
else
    if ! BONSOL_REQUESTER_KEYPAIR="$EXECUTION_KEYPAIR" \
        BONSOL_PAYER_KEYPAIR="$PAYER_KEYPAIR" \
        "$BONSOL_CMD" execute -f "$INPUT_PATH" \
        --wait; then
        echo "Error: Execution failed!"
        echo "Run with --debug flag for more information."
        exit 1
    fi
fi

echo "Execution complete! Check the output above for your I Ching reading."

# Restore original Solana config
if ! solana config set --keypair "$ORIGINAL_KEYPAIR"; then
    echo "Warning: Failed to restore original Solana config"
fi
