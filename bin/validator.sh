#!/usr/bin/env bash

set -e
export COPYFILE_DISABLE=1

print_usage() {
    echo "Usage: $0 [--bpf-program <address> <path>]... [-r]"
    echo "  --bpf-program: Add a BPF program with its address and path"
    echo "  -r: Run with '--reset' option (optional)"
    echo "sample usage: ./validator.sh -r --bpf-program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d ../../mainnet_bpf_programs/core.so"
}

extra_programs=()
reset_option=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --bpf-program)
            if [[ $# -lt 3 ]]; then
                echo "Error: --bpf-program requires two arguments: <address> <path>"
                print_usage
                exit 1
            fi
            extra_programs+=("--bpf-program" "$2" "$3")
            shift 3
            ;;
        -r)
            reset_option="-r"
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            print_usage
            exit 1
            ;;
    esac
done

set -x

if [ ! -x $(which cargo) ]; then
    echo "Rust and cargo must be installed"
    exit 1
fi

# Export dev mode for builds
export RISC0_DEV_MODE=1

# Set comprehensive logging to capture all program output
export RUST_LOG="error,\
solana_program::log=info,\
bonsol=debug,\
solana_runtime::message_processor=info,\
solana_runtime::transaction_processor=debug,\
solana_runtime::loader_utils=debug,\
solana_bpf_loader_program=debug"

export RUST_BACKTRACE=1

# Build all programs
echo "Building programs..."
(cd onchain/bonsol && cargo build-sbf)
(cd onchain/8bitoracle-iching-callback && cargo build-sbf)
(cd onchain/example-program-on-bonsol && cargo build-sbf)

# Get the program ID from the keypair file
CALLBACK_PROGRAM_ID=$(solana-keygen pubkey onchain/8bitoracle-iching-callback/scripts/program-keypair.json)
if [ -z "$CALLBACK_PROGRAM_ID" ]; then
    echo "Error: Could not get program ID from keypair file"
    exit 1
fi

# solana-test-validator \
#     --limit-ledger-size 0 \
#     --bind-address 0.0.0.0 \
#     --rpc-pubsub-enable-block-subscription \
#     --log \
#     --bpf-program BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew target/deploy/bonsol.so \
#     --bpf-program "$CALLBACK_PROGRAM_ID" target/deploy/bitoracle_iching_callback.so \
#     --bpf-program exay1T7QqsJPNcwzMiWubR6vZnqrgM16jZRraHgqBGG target/deploy/callback_example.so \
#     "${extra_programs[@]}" \
#     $reset_option

solana-test-validator \
    --limit-ledger-size 0 \
    --bind-address 0.0.0.0 \
    --rpc-pubsub-enable-block-subscription \
    --log \
    --bpf-program BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew target/deploy/bonsol.so \
    --bpf-program "$CALLBACK_PROGRAM_ID" target/deploy/bitoracle_iching_callback.so \
    --bpf-program exay1T7QqsJPNcwzMiWubR6vZnqrgM16jZRraHgqBGG target/deploy/callback_example.so \
    "${extra_programs[@]}" \
    $reset_option
