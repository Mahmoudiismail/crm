
param(
    [string]$OutlookHelperPath, [string]$Mode, [string]$To, [string]$Cc,
    [string]$Subject, [string]$HtmlBodyPath, [string]$AttachmentListPath,
    [string]$DisplayOrSend, [string]$BodyMode,
    [string]$SenderAccount, [string]$SubjectPrefix, [string]$AdditionalTo,
    [string]$RemoveCc, [string]$SendAtNextSevenAm
)
try {
    $ErrorActionPreference = "Stop"
    . $OutlookHelperPath
    $attachments = @()
    if ($AttachmentListPath) {
        if (-not (Test-Path -LiteralPath $AttachmentListPath -PathType Leaf)) { throw "Attachment list file not found: $AttachmentListPath" }
        $attachments = @(Get-Content -LiteralPath $AttachmentListPath -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    }
    $sendAtSeven = $SendAtNextSevenAm -eq "true"
    Send-OutlookMail -Mode $Mode -To $To -Cc $Cc -Subject $Subject -HtmlBodyPath $HtmlBodyPath -AttachmentPaths $attachments -DisplayOrSend $DisplayOrSend -BodyMode $BodyMode -SenderAccount $SenderAccount -SubjectPrefix $SubjectPrefix -AdditionalTo $AdditionalTo -RemoveCc $RemoveCc -SendAtNextSevenAm:$sendAtSeven
} catch {
    Write-Error "Outlook operation failed: $_"
    exit 1
}
