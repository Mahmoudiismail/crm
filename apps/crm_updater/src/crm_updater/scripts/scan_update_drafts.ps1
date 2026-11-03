
param(
    [string]$DownloadsDir,
    [string]$ResultPath
)

$ErrorActionPreference = 'Stop'
try {
    $Outlook = [Runtime.Interopservices.Marshal]::GetActiveObject("Outlook.Application")
} catch {
    $Outlook = New-Object -ComObject Outlook.Application
}

$Namespace = $Outlook.GetNamespace("MAPI")
$Drafts = $Namespace.GetDefaultFolder(16) # olFolderDrafts

$TargetItem = $null
$TargetAttachment = $null

foreach ($Item in $Drafts.Items) {
    if ($Item.Attachments.Count -gt 0) {
        foreach ($Attachment in $Item.Attachments) {
            if ($Attachment.FileName -match "^crm_tool_.*\.zip$") {
                $TargetItem = $Item
                $TargetAttachment = $Attachment
                break
            }
        }
    }
    if ($TargetItem) { break }
}

if ($TargetItem -and $TargetAttachment) {
    $SavePath = Join-Path $DownloadsDir $TargetAttachment.FileName
    $TargetAttachment.SaveAsFile($SavePath)
    Set-Content -LiteralPath $ResultPath -Value "FOUND:$SavePath" -Encoding UTF8
    $TargetItem.Delete()
} else {
    Set-Content -LiteralPath $ResultPath -Value "NOT_FOUND" -Encoding UTF8
}
