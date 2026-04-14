#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "CRM Tool — Build Windows executables"
echo "=========================================="
echo ""

docker compose run --rm build-windows

echo ""
echo "Done. Binaries: ./runner.exe and ./crm.exe"
