#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "CRM Tool - Docker Build (Windows executables)"
echo "=========================================="

# Build the Docker image
docker build -t crm-tool-builder .

# Extract Windows binaries
echo "[*] Extracting runner.exe and crm.exe..."
CONTAINER_ID=$(docker create crm-tool-builder)
docker cp "$CONTAINER_ID:/output/runner.exe" ./runner.exe
docker cp "$CONTAINER_ID:/output/crm.exe" ./crm.exe
docker rm "$CONTAINER_ID" > /dev/null

echo ""
echo "[✓] Done!"
ls -lh runner.exe crm.exe
