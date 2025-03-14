#!/bin/bash
set -e

# Get the hexagram PDA from input.json
HEXAGRAM_PDA=$(jq -r ".callbackConfig.extraAccounts[1].pubkey" input.json)

echo "Reading hexagram data from account: $HEXAGRAM_PDA"
solana account "$HEXAGRAM_PDA" --output json
