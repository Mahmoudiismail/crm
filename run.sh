#!/usr/bin/env bash
set -euo pipefail

echo "=========================================="
echo "CRM Tool — Build Windows .exe"
echo "=========================================="
echo ""

docker compose run --rm build-windows

echo ""
echo "Done. Binary: ./crm_tool.exe"
