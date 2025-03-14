#!/usr/bin/env bash

set -e
set -x
export COPYFILE_DISABLE=1

cleanup_validator() {
    echo "Cleaning up validator state..."
    pkill solana-test-validator || true
    pkill bonsol-node || true
    rm -rf test-ledger/
    rm -rf accounts/
    echo "Cleanup complete"
}

print_usage() {
    echo "Usage: $0 [--bpf-program <address> <path>]... [-r] [-d]"
    echo "  --bpf-program: Add a BPF program with its address and path"
    echo "  -r: Run with '--reset' option and clean up old validator state"
    echo "  -d: Show all debug output (optional)"
    echo "sample usage: ./validator.sh -r --bpf-program CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d ../../mainnet_bpf_programs/core.so"
}

extra_programs=()
reset_option=""
debug_mode=false

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
    -d)
        debug_mode=true
        shift
        ;;
    -h | --help)
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

if [ ! -x $(which cargo) ]; then
    echo "Rust and cargo must be installed"
    exit 1
fi

# Set comprehensive logging to capture all program output
export RUST_LOG="error,\
solana_metrics::metrics=error,\
solana_program::log=info,\
solana_runtime::message_processor::stable_log=info"

# Export dev mode for both builds
export RISC0_DEV_MODE=1

# If reset option is set, clean up first
if [ -n "$reset_option" ]; then
    cleanup_validator
fi

echo "Starting Solana test validator..."
solana-test-validator \
    --limit-ledger-size 0 \
    --bind-address 0.0.0.0 \
    --rpc-pubsub-enable-block-subscription \
    --log \
    --bpf-program BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew target/deploy/bonsol.so \
    --bpf-program 2gPzr1AjyYT8JqAndyTDMDUsQsH8y3tc9CuKUtKA2Uv1 target/deploy/bitoracle_iching_callback.so \
    --bpf-program exay1T7QqsJPNcwzMiWubR6vZnqrgM16jZRraHgqBGG target/deploy/callback_example.so \
    "${extra_programs[@]}" \
    $reset_option
