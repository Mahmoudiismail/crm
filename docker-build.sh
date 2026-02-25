#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "CRM Tool - Docker Build (Windows .exe)"
echo "=========================================="

# Build the Docker image
docker build -t crm-tool-builder .

# Extract Windows binary
echo "[*] Extracting crm_tool.exe..."
CONTAINER_ID=$(docker create crm-tool-builder)
docker cp "$CONTAINER_ID:/output/crm_tool.exe" ./crm_tool.exe
docker rm "$CONTAINER_ID" > /dev/null

echo ""
echo "[✓] Done!"
ls -lh crm_tool.exe
