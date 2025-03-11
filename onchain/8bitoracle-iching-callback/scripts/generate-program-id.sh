#!/bin/bash

# Exit on error
set -e

# Create keypair file if it doesn't exist
KEYPAIR_FILE="program-keypair.json"

if [ ! -f "$KEYPAIR_FILE" ]; then
  echo "Generating new program keypair..."
  solana-keygen new --no-bip39-passphrase -o "$KEYPAIR_FILE"
fi

# Get the program ID
PROGRAM_ID=$(solana-keygen pubkey "$KEYPAIR_FILE")
echo "Program ID: $PROGRAM_ID"

# Update the validator script with the actual program ID
echo "Updating validator script..."
sed -i "s/8bit1234567890123456789012345678901234567890123/$PROGRAM_ID/" ../../../bin/validator.sh

# Update the input generation script
echo "Updating input generation script..."
sed -i "s/8bit1234567890123456789012345678901234567890123/$PROGRAM_ID/" ../../../images/8bitoracle-iching/scripts/03-generate-input-with-callback.sh

echo "Done! Use this program ID in your deployment and configuration."
