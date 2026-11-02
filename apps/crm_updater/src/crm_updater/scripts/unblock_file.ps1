param(
    [Parameter(Mandatory = $true)]
    [string]$LiteralPath
)

$ErrorActionPreference = 'Stop'
Unblock-File -LiteralPath $LiteralPath
