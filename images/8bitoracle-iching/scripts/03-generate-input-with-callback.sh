#!/bin/bash

# Exit on error
set -e

# Debug: Print current directory and script location
echo "Current directory: $(pwd)"
echo "Script location: $0"

# Get project root directory (3 levels up from script location)
PROJECT_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"

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

# Get the image ID from manifest.json
MANIFEST_FILE="$(dirname "$0")/../manifest.json"
IMAGE_ID=$(grep -o '"imageId": "[^"]*' "$MANIFEST_FILE" | cut -d'"' -f4)
if [ -z "$IMAGE_ID" ]; then
  echo "Error: Could not find image ID in manifest.json"
  exit 1
fi
echo "Found image ID: $IMAGE_ID"

# Get the program ID from the keypair file
KEYPAIR_FILE="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/program-keypair.json"
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

# Get the public key of the requester (execution keypair)
REQUESTER=$(solana-keygen pubkey "$EXECUTION_KEYPAIR")
if [ -z "$REQUESTER" ]; then
  echo "Error: Could not get requester public key from keypair"
  exit 1
fi
echo "Using requester: $REQUESTER"

# Set this keypair as the default for Solana
solana config set --keypair "$EXECUTION_KEYPAIR"

# Generate a random execution ID
EXECUTION_ID=$(openssl rand -hex 16)
echo "Generated execution ID: $EXECUTION_ID"

# Get the bonsol program ID
BONSOL_PROGRAM_ID="BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew"

# After getting EXECUTION_ID, derive PDAs
echo "Deriving PDAs..."
echo "Using:"
echo "  Callback Program ID: $CALLBACK_PROGRAM_ID"
echo "  Requester: $REQUESTER"
echo "  Bonsol Program ID: $BONSOL_PROGRAM_ID"
echo "  Execution ID: $EXECUTION_ID"

PDA_SCRIPT="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/derive-pda.ts"
cd "$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts"
PDA_INFO=$(ts-node derive-pda.ts "$CALLBACK_PROGRAM_ID" "$REQUESTER" "$BONSOL_PROGRAM_ID" "$EXECUTION_ID" 2>&1)
DERIVE_EXIT=$?
cd - >/dev/null

# Print full PDA derivation output for debugging
echo "PDA derivation output:"
echo "$PDA_INFO"

if [ $DERIVE_EXIT -ne 0 ]; then
  echo "Error: PDA derivation failed"
  exit 1
fi

# Extract both PDAs from the output (now one per line)
EXECUTION_PDA=$(echo "$PDA_INFO" | grep -A1 "Final Results" | grep "Execution PDA:" | cut -d' ' -f3)
HEXAGRAM_PDA=$(echo "$PDA_INFO" | grep -A2 "Final Results" | grep "Hexagram PDA:" | cut -d' ' -f3)

if [ -z "$EXECUTION_PDA" ] || [ -z "$HEXAGRAM_PDA" ]; then
  echo "Error: Could not derive PDAs"
  echo "PDA script output:"
  echo "$PDA_INFO"
  exit 1
fi

echo "Derived PDAs:"
echo "  Execution PDA: $EXECUTION_PDA"
echo "  Hexagram PDA: $HEXAGRAM_PDA"

# Generate a random seed for the I Ching reading
RANDOM_SEED=$(openssl rand -hex 32)
echo "Generated random seed: 0x$RANDOM_SEED"

# Add timestamp for verification
TIMESTAMP=$(date +%s)
echo "Generated timestamp: $TIMESTAMP"

# Generate a random storage account keypair
STORAGE_KEYPAIR="$PROJECT_ROOT/onchain/8bitoracle-iching-callback/scripts/storage-keypair.json"
if [ ! -f "$STORAGE_KEYPAIR" ]; then
  solana-keygen new --no-bip39-passphrase -o "$STORAGE_KEYPAIR"
fi

# Get the public key of the storage account
STORAGE_PUBKEY=$(solana-keygen pubkey "$STORAGE_KEYPAIR")
if [ -z "$STORAGE_PUBKEY" ]; then
  echo "Error: Could not get storage public key from keypair"
  exit 1
fi
echo "Using storage account: $STORAGE_PUBKEY"

# Create input.json with callback configuration
INPUT_FILE="$PROJECT_ROOT/images/8bitoracle-iching/input.json"
echo "Creating input.json..."

# Backup existing input.json if it exists
if [ -f "$INPUT_FILE" ]; then
  BACKUP_FILE="${INPUT_FILE}.$(date +%Y%m%d_%H%M%S).bak"
  cp "$INPUT_FILE" "$BACKUP_FILE"
fi

# Create new input.json
jq -n \
  --arg timestamp "$TIMESTAMP" \
  --arg imageId "$IMAGE_ID" \
  --arg executionId "$EXECUTION_ID" \
  --arg randomSeed "$RANDOM_SEED" \
  --arg programId "$CALLBACK_PROGRAM_ID" \
  --arg executionPda "$EXECUTION_PDA" \
  --arg hexagramPda "$HEXAGRAM_PDA" \
  --arg storagePubkey "$STORAGE_PUBKEY" \
  '{
    "timestamp": ($timestamp | tonumber),
    "imageId": $imageId,
    "executionId": $executionId,
    "executionConfig": {
      "verifyInputHash": false,
      "forwardOutput": true
    },
    "inputs": [
      {
        "inputType": "PublicData",
        "data": ("0x" + $randomSeed)
      }
    ],
    "tip": 12000,
    "expiry": 1000,
    "preInstructions": [
      {
        "programId": "ComputeBudget111111111111111111111111111111",
        "accounts": [],
        "data": [
          0,
          88,
          21,
          0,
          0,
          0,
          0,
          0
        ]
      },
      {
        "programId": "ComputeBudget111111111111111111111111111111",
        "accounts": [],
        "data": [
          1,
          160,
          134,
          1,
          0,
          0,
          0,
          0
        ]
      }
    ],
    "callbackConfig": {
      "programId": $programId,
      "instructionPrefix": [1],
      "extraAccounts": [
        {
          "pubkey": $executionPda,
          "isSigner": true,
          "isWritable": false
        },
        {
          "pubkey": $hexagramPda,
          "isSigner": false,
          "isWritable": true
        },
        {
          "pubkey": "11111111111111111111111111111111",
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
echo "  Storage Account: $STORAGE_PUBKEY"
echo "You can now run 04-execute.sh to execute the I Ching program"
