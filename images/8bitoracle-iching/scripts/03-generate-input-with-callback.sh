#!/bin/bash

# Exit on error and enable debug tracing
set -e
set -x  # Add debug tracing

# Constants
FUNDING_AMOUNT="1"
MINIMUM_LAMPORTS=1000000000  # 1 SOL in lamports

# Function to check if a command exists
check_dependency() {
    if ! command -v "$1" &>/dev/null; then
        echo "Error: $1 is required but not installed."
        echo "Please install $1 first"
        exit 1
    fi
}

# Function to check and install npm dependencies
check_npm_deps() {
    local dir="$1"
    if [ ! -d "$dir/node_modules" ]; then
        echo "Installing npm dependencies in $dir..."
        cd "$dir"
        npm install
        npm install --save-dev @types/node
        cd - > /dev/null
    fi
}

# Check required dependencies
echo "Checking dependencies..."
REQUIRED_DEPS=("ts-node" "jq" "openssl" "solana-keygen" "solana" "npm")
for dep in "${REQUIRED_DEPS[@]}"; do
    check_dependency "$dep"
done
echo "✓ All dependencies found"

# Store original directory
ORIGINAL_DIR=$(pwd)

# Debug: Print current directory and script location
echo "Current directory: $ORIGINAL_DIR"
echo "Script location: $0"

# Get project root directory (3 levels up from script location)
PROJECT_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
echo "Project root: $PROJECT_ROOT"

# Source environment variables
ENV_FILE="$(dirname "$0")/../.env"
if [ -f "$ENV_FILE" ]; then
    echo "Loading environment variables from $ENV_FILE"
    set -a
    source "$ENV_FILE"
    set +a
else
    echo "Warning: .env file not found at $ENV_FILE"
fi

# Get the image ID from manifest.json using jq
MANIFEST_FILE="$(dirname "$0")/../manifest.json"
if [ ! -f "$MANIFEST_FILE" ]; then
    echo "Error: manifest.json not found at $MANIFEST_FILE"
    exit 1
fi

IMAGE_ID=$(jq -r '.imageId' "$MANIFEST_FILE")
if [ -z "$IMAGE_ID" ] || [ "$IMAGE_ID" = "null" ]; then
    echo "Error: Could not find image ID in manifest.json"
    exit 1
fi
echo "Found image ID: $IMAGE_ID"

# Get the program ID from the keypair file
KEYPAIR_FILE="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/program-keypair.json"

# Check if keypair file exists, if not create it
if [ ! -f "$KEYPAIR_FILE" ]; then
    echo "Program keypair not found at $KEYPAIR_FILE"
    echo "Generating new program keypair..."
    
    # Ensure directory exists
    mkdir -p "$(dirname "$KEYPAIR_FILE")"
    
    # Generate new keypair without a passphrase
    solana-keygen new --no-bip39-passphrase -o "$KEYPAIR_FILE"
    
    echo "Generated new program keypair at: $KEYPAIR_FILE"
    echo "⚠️  Important: You will need to deploy the program using this keypair"
    echo "   Run the deploy script after this completes"
fi

CALLBACK_PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_FILE")
if [ -z "$CALLBACK_PROGRAM_ID" ]; then
    echo "Error: Could not get program ID from keypair file"
    exit 1
fi
echo "Found program ID: $CALLBACK_PROGRAM_ID"

# Get the test execution keypair path
EXECUTION_KEYPAIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/test-execution-keypair.json"
if [ ! -f "$EXECUTION_KEYPAIR" ]; then
  echo "Error: Test execution keypair not found at $EXECUTION_KEYPAIR"
  exit 1
fi

# Create a separate payer keypair
PAYER_KEYPAIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/test-payer-keypair.json"
if [ ! -f "$PAYER_KEYPAIR" ]; then
  echo "Generating new payer keypair..."
  solana-keygen new --no-bip39-passphrase -o "$PAYER_KEYPAIR"
  echo "Generated new payer keypair at: $PAYER_KEYPAIR"
fi

# Export the BONSOL keypair environment variables
export BONSOL_REQUESTER_KEYPAIR="$EXECUTION_KEYPAIR"
export BONSOL_PAYER_KEYPAIR="$PAYER_KEYPAIR"
echo "Set BONSOL keypair environment variables:"
echo "BONSOL_REQUESTER_KEYPAIR=$BONSOL_REQUESTER_KEYPAIR"
echo "BONSOL_PAYER_KEYPAIR=$BONSOL_PAYER_KEYPAIR"

# Write exports to a file that can be sourced later
EXPORT_FILE="$(dirname "$0")/bonsol_exports.sh"
echo "Writing environment exports to: $EXPORT_FILE"
cat > "$EXPORT_FILE" << EOF
export BONSOL_REQUESTER_KEYPAIR="$EXECUTION_KEYPAIR"
export BONSOL_PAYER_KEYPAIR="$PAYER_KEYPAIR"
EOF

echo "Environment variables have been written to $EXPORT_FILE"
echo "To use them in your current shell, run:"
echo "  source $EXPORT_FILE"

# Get the public key of the payer
PAYER=$(solana-keygen pubkey "$PAYER_KEYPAIR")
if [ -z "$PAYER" ]; then
  echo "Error: Could not get payer public key from keypair"
  exit 1
fi
echo "Using payer: $PAYER"

# Store current config
echo "Storing current Solana config..."
ORIGINAL_KEYPAIR=$(solana config get | grep "Keypair Path" | awk '{print $3}')
echo "Original keypair: $ORIGINAL_KEYPAIR"

# Check payer balance and handle airdrop if needed
echo "Checking payer account balance..."
PAYER_BALANCE_SOL=$(solana balance "$PAYER" | awk '{print $1}')
PAYER_BALANCE=$(echo "$PAYER_BALANCE_SOL * 1000000000" | bc | cut -d'.' -f1)

if [ -z "$PAYER_BALANCE" ] || [ "$PAYER_BALANCE" -lt "$MINIMUM_LAMPORTS" ]; then
  echo "Payer balance too low, attempting to airdrop 10 SOL..."
  
  # Get current cluster
  CLUSTER=$(solana config get | grep "RPC URL" | awk '{print $3}')
  if [[ "$CLUSTER" == *"mainnet"* ]]; then
    echo "Error: Insufficient funds on mainnet. Please fund payer account manually."
    exit 1
  fi
  
  # Try airdrop up to 3 times
  for i in {1..3}; do
    if solana airdrop 10 "$PAYER"; then
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
  
  # Verify funding was successful
  PAYER_BALANCE_SOL=$(solana balance "$PAYER" | awk '{print $1}')
  PAYER_BALANCE=$(echo "$PAYER_BALANCE_SOL * 1000000000" | bc | cut -d'.' -f1)
  
  if [ "$PAYER_BALANCE" -lt "$MINIMUM_LAMPORTS" ]; then
    echo "Error: Payer account funding verification failed"
    echo "Current balance: $PAYER_BALANCE lamports"
    echo "Required minimum: $MINIMUM_LAMPORTS lamports"
    exit 1
  fi
fi

echo "Payer account funded successfully with $PAYER_BALANCE_SOL SOL"

# Get the public key of the requester (execution keypair)
REQUESTER=$(solana-keygen pubkey "$EXECUTION_KEYPAIR")
if [ -z "$REQUESTER" ]; then
  echo "Error: Could not get requester public key from keypair"
  exit 1
fi
echo "Using requester: $REQUESTER"

# Generate a random execution ID
EXECUTION_ID=$(openssl rand -hex 16)
echo "Generated execution ID: $EXECUTION_ID"

# Get the bonsol program ID
BONSOL_PROGRAM_ID="BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew"

# After getting EXECUTION_ID, derive PDAs
echo "Deriving PDAs..."
echo "Using:"
echo "  Callback Program ID: $CALLBACK_PROGRAM_ID"
echo "  Payer: $PAYER"
echo "  Bonsol Program ID: $BONSOL_PROGRAM_ID"
echo "  Image ID: $IMAGE_ID"
echo "  Execution ID: $EXECUTION_ID"

PDA_SCRIPT="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/derive-pda.ts"
if [ ! -f "$PDA_SCRIPT" ]; then
    echo "Error: PDA derivation script not found at $PDA_SCRIPT"
    exit 1
fi

# Before running ts-node, ensure dependencies are installed
SCRIPTS_DIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts"
check_npm_deps "$SCRIPTS_DIR"

# Change to scripts directory for ts-node
cd "$SCRIPTS_DIR"

# Run ts-node and capture stdout and stderr separately
PDA_INFO_ERR=$(ts-node derive-pda.ts "$CALLBACK_PROGRAM_ID" "$PAYER" "$BONSOL_PROGRAM_ID" "$EXECUTION_ID" "$IMAGE_ID" 2>&1 >/dev/null)
PDA_INFO=$(ts-node derive-pda.ts "$CALLBACK_PROGRAM_ID" "$PAYER" "$BONSOL_PROGRAM_ID" "$EXECUTION_ID" "$IMAGE_ID" 2>/dev/null)
DERIVE_EXIT=$?

# Return to original directory
cd "$ORIGINAL_DIR"

# Print debug output
echo "PDA derivation debug output:"
echo "$PDA_INFO_ERR"

if [ $DERIVE_EXIT -ne 0 ]; then
    echo "Error: PDA derivation failed"
    echo "Full error output:"
    echo "$PDA_INFO_ERR"
    exit 1
fi

# Extract PDAs from stdout (now has three lines)
EXECUTION_PDA=$(echo "$PDA_INFO" | head -n1)
HEXAGRAM_PDA=$(echo "$PDA_INFO" | head -n2 | tail -n1)
DEPLOYMENT_PDA=$(echo "$PDA_INFO" | tail -n1)

# Validate PDA format (should be base58 encoded, 32-44 characters)
PDA_REGEX='^[1-9A-HJ-NP-Za-km-z]{32,44}$'
if [ -z "$EXECUTION_PDA" ] || [ -z "$HEXAGRAM_PDA" ] || [ -z "$DEPLOYMENT_PDA" ] || \
   ! [[ $EXECUTION_PDA =~ $PDA_REGEX ]] || \
   ! [[ $HEXAGRAM_PDA =~ $PDA_REGEX ]] || \
   ! [[ $DEPLOYMENT_PDA =~ $PDA_REGEX ]]; then
    echo "Error: Invalid PDA format"
    echo "Execution PDA: $EXECUTION_PDA"
    echo "Hexagram PDA: $HEXAGRAM_PDA"
    echo "Deployment PDA: $DEPLOYMENT_PDA"
    echo "PDAs should be base58 encoded and 32-44 characters long"
    exit 1
fi

echo "Derived PDAs:"
echo "  Execution PDA: $EXECUTION_PDA"
echo "  Hexagram PDA: $HEXAGRAM_PDA"
echo "  Deployment PDA: $DEPLOYMENT_PDA"

# Calculate required space for HexagramData
# - 6 bytes for lines [u8; 6]
# - 47 bytes for ascii_art [u8; ASCII_ART_SIZE]
# - 8 bytes for timestamp i64
# - 1 byte for is_initialized bool
HEXAGRAM_SPACE=$((6 + 47 + 8 + 1))
echo "Required space for HexagramData: $HEXAGRAM_SPACE bytes"

# Build the init-storage CLI tool if needed
echo "Building init-storage CLI tool..."
cargo build --manifest-path "$PROJECT_ROOT/onchain/8bitoracle-iching-callback/cli/init-storage/Cargo.toml" || {
    echo "Error: Failed to build init-storage CLI tool"
    exit 1
}

# Initialize the storage account using our CLI tool
echo "Initializing storage account..."
"$PROJECT_ROOT/target/debug/init-storage" \
    --storage-address "$HEXAGRAM_PDA" \
    --payer "$PAYER" \
    --keypair "$PAYER_KEYPAIR" || {
    echo "Warning: Storage account initialization failed (may already exist)"
}

# Wait for transaction confirmation
sleep 2

# Verify the account exists and has correct ownership
echo "Verifying storage account..."
STORAGE_INFO=$(solana account "$HEXAGRAM_PDA")
if [ $? -ne 0 ]; then
    echo "Error: Failed to fetch storage account info"
    exit 1
fi

STORAGE_OWNER=$(echo "$STORAGE_INFO" | grep "Owner:" | awk '{print $2}')

echo "Storage account info:"
echo "  Owner: $STORAGE_OWNER"

# Verify the owner is the callback program
if [ "$STORAGE_OWNER" != "$CALLBACK_PROGRAM_ID" ]; then
    echo "Error: Storage account has incorrect owner"
    echo "Expected: $CALLBACK_PROGRAM_ID"
    echo "Got: $STORAGE_OWNER"
    exit 1
fi

echo "✓ Storage account initialized with correct program ownership"

# Fund execution PDA
echo "Funding execution PDA..."
if ! solana transfer --allow-unfunded-recipient "$EXECUTION_PDA" "$FUNDING_AMOUNT"; then
    echo "Warning: Failed to fund execution PDA (may already be funded)"
fi

# Verify account funding
echo "Verifying account funding..."
EXECUTION_BALANCE_SOL=$(solana balance "$EXECUTION_PDA" | awk '{print $1}')
EXECUTION_BALANCE=$(echo "$EXECUTION_BALANCE_SOL * 1000000000" | bc | cut -d'.' -f1)

echo "Current balances:"
echo "  Execution PDA ($EXECUTION_PDA): $EXECUTION_BALANCE lamports ($EXECUTION_BALANCE_SOL SOL)"

if [ "$EXECUTION_BALANCE" -lt "$MINIMUM_LAMPORTS" ]; then
    echo "Error: Account funding verification failed"
    echo "Required minimum: $MINIMUM_LAMPORTS lamports ($FUNDING_AMOUNT SOL)"
    echo "Current balances:"
    echo "  Execution PDA: $EXECUTION_BALANCE lamports ($EXECUTION_BALANCE_SOL SOL)"
    exit 1
fi

echo "✓ Accounts successfully funded"

# Generate a random seed for the I Ching reading
RANDOM_SEED=$(openssl rand -hex 32)
echo "Generated random seed: 0x$RANDOM_SEED"

# Add timestamp for verification
TIMESTAMP=$(date +%s)
echo "Generated timestamp: $TIMESTAMP"

# Get current block height and calculate expiry
echo "Checking current slot and calculating expiry..."
CURRENT_SLOT=$(solana slot)
if [ -z "$CURRENT_SLOT" ]; then
    echo "Error: Could not get current slot"
    exit 1
fi

# Validate that slot is reasonable
if [ "$CURRENT_SLOT" -lt 1 ]; then
    echo "Error: Current slot ($CURRENT_SLOT) is invalid"
    exit 1
fi

# Calculate expiry with a minimum window
MIN_EXPIRY_WINDOW=1000
EXPIRY_WINDOW=1000000

# Add buffer to account for potential slot changes
BUFFER_BLOCKS=100
EXPIRY_WINDOW=$((EXPIRY_WINDOW + BUFFER_BLOCKS))

echo "Expiry calculation:"
echo "  Current slot: $CURRENT_SLOT"
echo "  Minimum expiry window: $MIN_EXPIRY_WINDOW"
echo "  Buffer blocks: $BUFFER_BLOCKS"
echo "  Desired expiry window: $EXPIRY_WINDOW"

# Ensure expiry window is at least the minimum
if [ "$EXPIRY_WINDOW" -lt "$MIN_EXPIRY_WINDOW" ]; then
    echo "Warning: Expiry window is less than minimum, using minimum value"
    EXPIRY_WINDOW=$MIN_EXPIRY_WINDOW
fi

MAX_BLOCK_HEIGHT=$((CURRENT_SLOT + EXPIRY_WINDOW))

echo "Final expiry configuration:"
echo "  Current slot: $CURRENT_SLOT"
echo "  Expiry window: $EXPIRY_WINDOW blocks"
echo "  Max block height: $MAX_BLOCK_HEIGHT"
echo "  Time until expiry: ~$((EXPIRY_WINDOW / 2)) seconds (assuming 2 slots/sec)"

# Validate the calculated max block height
if [ "$MAX_BLOCK_HEIGHT" -le "$CURRENT_SLOT" ]; then
    echo "Error: Invalid max block height calculation"
    echo "Max block height ($MAX_BLOCK_HEIGHT) must be greater than current slot ($CURRENT_SLOT)"
    exit 1
fi

# Get the prover account (using the Bonsol program ID)
PROVER_PUBKEY="$BONSOL_PROGRAM_ID"  # Use Bonsol Program ID as prover
if [ -z "$PROVER_PUBKEY" ]; then
  echo "Error: Could not get prover public key"
  exit 1
fi
echo "Using prover account: $PROVER_PUBKEY"

# Get the system program ID (constant)
SYSTEM_PROGRAM_ID="11111111111111111111111111111111"
echo "Using system program ID: $SYSTEM_PROGRAM_ID"

# Create input.json with callback configuration
INPUT_FILE="$PROJECT_ROOT/images/8bitoracle-iching/input.json"
echo "Creating input.json..."

# Backup existing input.json if it exists
if [ -f "$INPUT_FILE" ]; then
  BACKUP_FILE="${INPUT_FILE}.$(date +%Y%m%d_%H%M%S).bak"
  cp "$INPUT_FILE" "$BACKUP_FILE"
fi

# Force dev mode to be enabled
export RISC0_DEV_MODE=1

# Set dev mode flag
DEV_MODE=${RISC0_DEV_MODE:-0}
echo "Dev mode setting: $DEV_MODE"

# Create new input.json
jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg imageId "$IMAGE_ID" \
  --arg executionId "$EXECUTION_ID" \
  --arg randomSeed "$RANDOM_SEED" \
  --arg programId "$CALLBACK_PROGRAM_ID" \
  --arg hexagramPda "$HEXAGRAM_PDA" \
  --arg systemProgram "$SYSTEM_PROGRAM_ID" \
  --arg deploymentPda "$DEPLOYMENT_PDA" \
  --arg maxBlockHeight "$MAX_BLOCK_HEIGHT" \
  --arg executionPda "$EXECUTION_PDA" \
  --arg bonsolProgram "BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew" \
  --argjson devMode "$DEV_MODE" \
  '{
    "timestamp": ($timestamp | tonumber),
    "imageId": $imageId,
    "executionId": $executionId,
    "executionPda": $executionPda,
    "executionConfig": {
      "verifyInputHash": false,
      "forwardOutput": true,
      "devMode": $devMode
    },
    "inputs": [
      {
        "inputType": "PublicData",
        "data": ("0x" + $randomSeed)
      }
    ],
    "tip": 12000,
    "expiry": ($maxBlockHeight | tonumber),
    "callbackConfig": {
      "programId": $programId,
      "instructionPrefix": [0],
      "extraAccounts": [
        {
          "pubkey": $hexagramPda,
          "isSigner": false,
          "isWritable": true
        },
        {
          "pubkey": $systemProgram,
          "isSigner": false,
          "isWritable": false
        }
      ]
    }
  }' >"$INPUT_FILE"

echo "Created input.json at $INPUT_FILE"
cat "$INPUT_FILE"

echo "Successfully generated input.json at: $INPUT_FILE"
echo "Generated with:"
echo "  Execution ID: $EXECUTION_ID"
echo "  Execution PDA: $EXECUTION_PDA"
echo "  Hexagram PDA: $HEXAGRAM_PDA"
echo "  Prover Account: $PROVER_PUBKEY"
echo "You can now run 04-execute.sh to execute the I Ching program"

# Set Solana config to use the payer keypair
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

# Add explicit success exit
exit 0
