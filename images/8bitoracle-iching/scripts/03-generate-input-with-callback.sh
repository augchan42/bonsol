#!/bin/bash

# Exit on error
set -e

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
IMAGE_ID=$(grep -o '"imageId": "[^"]*' ../manifest.json | cut -d'"' -f4)
if [ -z "$IMAGE_ID" ]; then
    echo "Error: Could not find image ID in manifest.json"
    exit 1
fi

# Generate a random seed for the I Ching reading
RANDOM_SEED=$(openssl rand -hex 16)

# Generate a random execution ID for this reading
EXECUTION_ID=$(openssl rand -hex 16)

# Get the program ID from the keypair file
CALLBACK_PROGRAM_ID=$(solana-keygen pubkey ../../../onchain/8bitoracle-iching-callback/scripts/program-keypair.json)
if [ -z "$CALLBACK_PROGRAM_ID" ]; then
    echo "Error: Could not get program ID from keypair file"
    exit 1
fi

# For testing, we'll use a known keypair for the execution account
# In production, this would be the actual execution account
EXECUTION_KEYPAIR="../../../onchain/8bitoracle-iching-callback/scripts/test-execution-keypair.json"
if [ ! -f "$EXECUTION_KEYPAIR" ]; then
    echo "Generating test execution keypair..."
    solana-keygen new --no-bip39-passphrase -o "$EXECUTION_KEYPAIR"
fi

EXECUTION_ACCOUNT=$(solana-keygen pubkey "$EXECUTION_KEYPAIR")

# Derive the PDA for hexagram storage
echo "Deriving PDA for hexagram storage..."
PDA_INFO=$(cd ../../../onchain/8bitoracle-iching-callback/scripts && ts-node derive-pda.ts "$CALLBACK_PROGRAM_ID" "$EXECUTION_ACCOUNT")
HEXAGRAM_PDA=$(echo "$PDA_INFO" | grep "PDA:" | cut -d' ' -f2)

if [ -z "$HEXAGRAM_PDA" ]; then
    echo "Error: Could not derive PDA"
    exit 1
fi

echo "Using PDA: $HEXAGRAM_PDA for hexagram storage"

# Create input.json with callback configuration
cat >../input.json <<EOL
{
    "imageId": "${IMAGE_ID}",
    "executionId": "${EXECUTION_ID}",
    "executionConfig": {
        "verifyInputHash": false,
        "forwardOutput": true
    },
    "inputs": [
        {
            "inputType": "Private",
            "data": "${RANDOM_SEED}"
        }
    ],
    "tip": 12000,
    "expiry": 100,
    "callbackConfig": {
        "programId": "${CALLBACK_PROGRAM_ID}",
        "instructionPrefix": [1],
        "extraAccounts": [
            {
                "pubkey": "${EXECUTION_ACCOUNT}",
                "isSigner": true,
                "isWritable": true
            },
            {
                "pubkey": "${HEXAGRAM_PDA}",
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
}
EOL

echo "Generated input.json with 8BitOracle callback configuration:"
cat ../input.json
