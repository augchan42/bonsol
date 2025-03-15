#!/bin/bash

# Exit on error
set -e

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

# Check account balance and handle airdrop if needed
echo "Checking deployer account balance..."
DEPLOYER_PUBKEY=$(solana address)
if [ -z "$DEPLOYER_PUBKEY" ]; then
    echo "Error: Could not get deployer public key. Please check your Solana configuration."
    exit 1
fi
echo "Deployer account: $DEPLOYER_PUBKEY"

# Get current balance
BALANCE=$(solana balance "$DEPLOYER_PUBKEY" | awk '{print $1}')
echo "Current balance: $BALANCE SOL"

# Check if balance is less than 1 SOL
if (($(echo "$BALANCE < 1" | bc -l))); then
    echo "Balance too low for deployment"
    
    # Get current cluster
    CLUSTER=$(solana config get | grep "RPC URL" | awk '{print $3}')
    if [[ "$CLUSTER" == *"mainnet"* ]]; then
        echo "Error: Insufficient funds on mainnet. Please fund account manually."
        exit 1
    else
        echo "Attempting to airdrop 2 SOL..."
        # Try airdrop up to 3 times
        for i in {1..3}; do
            if solana airdrop 2 "$DEPLOYER_PUBKEY"; then
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
        NEW_BALANCE=$(solana balance "$DEPLOYER_PUBKEY" | awk '{print $1}')
        echo "New balance: $NEW_BALANCE SOL"
        
        if (($(echo "$NEW_BALANCE < 1" | bc -l))); then
            echo "Error: Balance still too low after airdrop. Please fund account manually."
            exit 1
        fi
    fi
fi

echo "Balance check passed ✓"
echo "----------------------------------------"

# Always enable RISC0_DEV_MODE for development/testing
export RISC0_DEV_MODE=1
echo "RISC0_DEV_MODE enabled for development/testing"

# Enable debug logging if --debug flag is passed
if [ "$DEBUG" = true ]; then
    echo "Debug mode enabled"
    export RUST_LOG="info,bonsol=debug,object_store=debug,solana_program::log=debug,bonsol_prover::input_resolver=debug,risc0_runner=debug"
    export RUST_BACKTRACE=1
    echo "Debug logging enabled for:"
    echo "  - Solana program logs"
    echo "  - RISC0 runner"
    echo "  - Bonsol prover"
    echo "  - Object store"
fi

# Validate required environment variables
if [ -z "$AWS_ACCESS_KEY_ID" ]; then
    echo "Error: AWS_ACCESS_KEY_ID is not set"
    exit 1
fi

if [ -z "$AWS_SECRET_ACCESS_KEY" ]; then
    echo "Error: AWS_SECRET_ACCESS_KEY is not set"
    exit 1
fi

# Ensure S3_ENDPOINT has https:// prefix and no trailing slash
if [ -n "$S3_ENDPOINT" ]; then
    # Remove any existing protocol and trailing slash
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT#https://}
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT_CLEAN#http://}
    S3_ENDPOINT_CLEAN=${S3_ENDPOINT_CLEAN%/}
    # Add https:// back
    S3_ENDPOINT_FULL="https://$S3_ENDPOINT_CLEAN"

    if [ "$DEBUG" = true ]; then
        echo "Debug: S3 Configuration:"
        echo "  Original endpoint: $S3_ENDPOINT"
        echo "  Cleaned endpoint: $S3_ENDPOINT_CLEAN"
        echo "  Final endpoint: $S3_ENDPOINT_FULL"
        echo "  Bucket: ${BUCKET:-8bitoracle}"
        echo "  Region: ${AWS_REGION:-us-east-1}"
    fi
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
            echo "Debug: Bonsol binary version:"
            "$BONSOL_CMD" --version
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
        echo "Debug: Bonsol binary details:"
        ls -l "$(which bonsol)"
        echo "Debug: Bonsol version:"
        "$BONSOL_CMD" --version
    fi
fi

# Store original directory
ORIGINAL_DIR=$(pwd)

# Change to the I Ching program directory
cd "$(dirname "$0")/.."
ICHING_DIR=$(pwd)
echo "Changed to I Ching directory: $ICHING_DIR"

# Build the program first
if [ "$USE_LOCAL" = true ]; then
    echo "Using local bonsol build: $BONSOL_CMD"
    if [ "$DEBUG" = true ]; then
        echo "Debug: Bonsol binary details:"
        ls -l "$BONSOL_CMD"
        echo "Debug: Bonsol binary last modified:"
        stat "$BONSOL_CMD"
    fi
else
    echo "Building bonsol from source"
    cargo build
fi

if [ "$DEBUG" = true ]; then
    echo "Debug: Manifest contents:"
    cat manifest.json
    echo ""
fi

echo "Deploying to S3..."
echo "Using endpoint: ${S3_ENDPOINT_FULL:-default AWS endpoint}"
echo "Using bucket: ${BUCKET:-8bitoracle}"

# Extract image ID before deployment
IMAGE_ID=$(grep -o '"imageId": "[^"]*' manifest.json | cut -d'"' -f4)

# Construct the full object key
OBJECT_KEY="iching-$IMAGE_ID"

# Set environment variables for deployment
export BONSOL_S3_BUCKET="${BUCKET:-8bitoracle}"
export BONSOL_S3_OBJECT_KEY="$OBJECT_KEY"

DEPLOY_CMD="$BONSOL_CMD deploy s3 \
    --bucket \"${BUCKET:-8bitoracle}\" \
    --access-key \"${AWS_ACCESS_KEY_ID}\" \
    --secret-key \"${AWS_SECRET_ACCESS_KEY}\" \
    --region \"${AWS_REGION:-us-east-1}\" \
    --manifest-path manifest.json \
    ${S3_ENDPOINT_FULL:+--endpoint \"$S3_ENDPOINT_FULL\"}"

if [ "$DEBUG" = true ]; then
    echo "Debug: Deploy command (with secrets redacted):"
    echo "$DEPLOY_CMD" | sed 's/--access-key "[^"]*"/--access-key "***"/g' | sed 's/--secret-key "[^"]*"/--secret-key "***"/g'
    echo "Debug: Object key: $OBJECT_KEY"
fi

eval "$DEPLOY_CMD"

echo
echo "Done! Program deployed successfully."
echo "Image ID (you'll need this for execution): $IMAGE_ID"

# Construct S3 URL based on endpoint configuration
if [ -n "$S3_ENDPOINT_FULL" ]; then
    S3_URL="$S3_ENDPOINT_FULL/${BUCKET:-8bitoracle}/$OBJECT_KEY"
else
    S3_URL="https://s3.${AWS_REGION:-us-east-1}.amazonaws.com/${BUCKET:-8bitoracle}/$OBJECT_KEY"
fi

echo "S3 URL: $S3_URL"

if [ "$DEBUG" = true ]; then
    echo "Debug: URL components:"
    echo "  Base URL: ${S3_ENDPOINT_FULL:-https://s3.${AWS_REGION:-us-east-1}.amazonaws.com}"
    echo "  Bucket: ${BUCKET:-8bitoracle}"
    echo "  Object key: $OBJECT_KEY"
    echo "  Full URL: $S3_URL"
    echo "  Environment variables:"
    echo "    BONSOL_S3_BUCKET=$BONSOL_S3_BUCKET"
    echo "    BONSOL_S3_OBJECT_KEY=$BONSOL_S3_OBJECT_KEY"
fi

# Return to original directory
cd "$ORIGINAL_DIR"

echo "You can now run 03-generate-input.sh with the image ID to prepare for execution."

# Usage hint if no arguments were provided
if [ "$#" -eq 0 ]; then
    echo
    echo "Note: You can use --local to run with a local bonsol build from target/debug/"
    echo "      You can use --debug to enable detailed logging"
fi
