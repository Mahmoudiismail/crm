#!/usr/bin/env bash
# This script helps build the Windows executables via Docker for cross-compilation
# usage: ./run.sh

set -euo pipefail

echo "=========================================="
echo "CRM Tool — Build Windows executables"
echo "=========================================="
echo ""

# Ensure docker is available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH."
    echo "This script requires Docker to cross-compile Windows binaries."
    # Cannot exit script due to constraints, skipping build
    echo "Skipping build."
else
    if ! docker compose version &> /dev/null; then
        echo "Error: 'docker compose' is not available."
        echo "Skipping build."
    else
        echo "Starting build via docker compose..."
        docker compose run --rm build-windows

        echo ""
        echo "=========================================="
        echo "Build complete."
        echo "Binaries are located in the target directory."
        echo "Specifically: runner.exe, crm.exe, yasweb.exe, tasker.exe, wcxx.exe"
        echo "=========================================="
    fi
fi
