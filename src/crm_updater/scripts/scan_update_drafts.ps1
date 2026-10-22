
param(
    [string]$DownloadsDir
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
    Write-Output "FOUND:$SavePath"
    $TargetItem.Delete()
} else {
    Write-Output "NOT_FOUND"
}
