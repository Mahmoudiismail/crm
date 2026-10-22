
function Get-OutlookSenderAddress($Item) {
    $senderAddress = ""
    try { $senderAddress = [string]$Item.SenderEmailAddress } catch {}
    if ($Item.SenderEmailType -eq "EX") {
        try {
            $exchangeUser = $Item.Sender.GetExchangeUser()
            if ($exchangeUser -and $exchangeUser.PrimarySmtpAddress) { $senderAddress = [string]$exchangeUser.PrimarySmtpAddress }
        } catch {}
        if ([string]::IsNullOrWhiteSpace($senderAddress) -or $senderAddress.IndexOf("@") -lt 0) {
            try { $senderAddress = [string]$Item.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x39FE001E") } catch {}
        }
    }
    return $senderAddress
}

function Find-OutlookOriginalMessage($Folder, [string]$SortProperty, [string]$SenderAccount, [string]$SubjectPrefix) {
    $items = $Folder.Items
    $items.Sort($SortProperty, $true)
    foreach ($item in $items) {
        if (-not $item.Subject -or -not $item.Subject.StartsWith($SubjectPrefix, [System.StringComparison]::OrdinalIgnoreCase)) { continue }
        $senderAddress = Get-OutlookSenderAddress $item
        if ([string]::Equals($senderAddress.Trim(), $SenderAccount.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) { return $item }
    }
    return $null
}

function Get-OutlookHtmlFragment([string]$Html) {
    $bodyStart = $Html.IndexOf("<body", [System.StringComparison]::OrdinalIgnoreCase)
    if ($bodyStart -lt 0) { return $Html }
    $bodyTagEnd = $Html.IndexOf(">", $bodyStart)
    if ($bodyTagEnd -lt 0) { return $Html }
    $bodyEnd = $Html.LastIndexOf("</body>", [System.StringComparison]::OrdinalIgnoreCase)
    if ($bodyEnd -lt $bodyTagEnd) { $bodyEnd = $Html.Length }
    return $Html.Substring($bodyTagEnd + 1, $bodyEnd - $bodyTagEnd - 1)
}

function Send-OutlookMail {
    param(
        [ValidateSet("New", "ReplyAll")][string]$Mode = "New",
        [string]$To = "", [string]$Cc = "", [string]$Subject = "",
        [string]$HtmlBodyPath = "", [string[]]$AttachmentPaths = @(),
        [ValidateSet("Send()", "Send", "Display")][string]$DisplayOrSend = "Send()",
        [string]$BodyMode = "Replace", [string]$SenderAccount = "",
        [string]$SubjectPrefix = "", [string]$AdditionalTo = "", [string]$RemoveCc = "",
        [switch]$SendAtNextSevenAm
    )
    $ErrorActionPreference = "Stop"
    $outlook = New-Object -ComObject Outlook.Application
    if ($Mode -eq "ReplyAll") {
        if ([string]::IsNullOrWhiteSpace($SenderAccount) -or [string]::IsNullOrWhiteSpace($SubjectPrefix)) { throw "ReplyAll requires SenderAccount and SubjectPrefix." }
        $namespace = $outlook.GetNamespace("MAPI")
        $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(6)) "[ReceivedTime]" $SenderAccount $SubjectPrefix
        if (-not $mail) { $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(5)) "[SentOn]" $SenderAccount $SubjectPrefix }
        if (-not $mail) { throw "Original message with subject prefix '$SubjectPrefix' not found in Inbox or Sent Items of '$SenderAccount'." }
        $mail = $mail.ReplyAll()
    } else {
        $mail = $outlook.CreateItem(0)
        if ($To) { $mail.To = $To }
        if ($Cc) { $mail.CC = $Cc }
    }
    if ($AdditionalTo -and $Mode -eq "ReplyAll") {
        $currentTo = [string]$mail.To
        if ($currentTo.IndexOf($AdditionalTo, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
            if ([string]::IsNullOrWhiteSpace($currentTo)) { $mail.To = $AdditionalTo }
            else { $mail.To = $currentTo.TrimEnd(';', ' ', ',') + "; " + $AdditionalTo }
        }
    }
    if ($RemoveCc -and $Mode -eq "ReplyAll") {
        $ccRecipients = @(([string]$mail.CC -split ';') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
        $hasRecipientToRemove = @($ccRecipients | Where-Object {
            $_.IndexOf($RemoveCc, [System.StringComparison]::OrdinalIgnoreCase) -ge 0
        }).Count -gt 0
        if ($hasRecipientToRemove) {
            $remainingCcRecipients = @($ccRecipients | Where-Object {
                $_.IndexOf($RemoveCc, [System.StringComparison]::OrdinalIgnoreCase) -lt 0
            })
            $mail.CC = $remainingCcRecipients -join "; "
        }
    }
    if ($Subject) { $mail.Subject = $Subject }
    if ($HtmlBodyPath) {
        if (-not (Test-Path -LiteralPath $HtmlBodyPath -PathType Leaf)) { throw "Email HTML body file not found: $HtmlBodyPath" }
        $htmlBody = Get-Content -LiteralPath $HtmlBodyPath -Raw -Encoding UTF8
        if ($BodyMode -eq "Prepend" -and $Mode -eq "ReplyAll") {
            $fragment = Get-OutlookHtmlFragment $htmlBody
            $existingHtml = [string]$mail.HTMLBody
            $bodyStart = $existingHtml.IndexOf("<body", [System.StringComparison]::OrdinalIgnoreCase)
            $bodyTagEnd = if ($bodyStart -ge 0) { $existingHtml.IndexOf(">", $bodyStart) } else { -1 }
            if ($bodyTagEnd -ge 0) { $mail.HTMLBody = $existingHtml.Insert($bodyTagEnd + 1, $fragment) }
            else { $mail.HTMLBody = $fragment + $existingHtml }
        } else { $mail.HTMLBody = $htmlBody }
    }
    foreach ($attachmentPath in $AttachmentPaths) {
        if ([string]::IsNullOrWhiteSpace($attachmentPath)) { continue }
        if (-not (Test-Path -LiteralPath $attachmentPath -PathType Leaf)) { throw "Email attachment not found: $attachmentPath" }
        $mail.Attachments.Add([System.IO.Path]::GetFullPath($attachmentPath)) | Out-Null
    }
    if ($SendAtNextSevenAm) {
        $now = Get-Date
        $targetTime = $now.Date.AddHours(7)
        if ($now -ge $targetTime) { $targetTime = $targetTime.AddDays(1) }
        $mail.DeferredDeliveryTime = $targetTime
    }
    if ($DisplayOrSend -eq "Display") { $mail.Display() } else { $mail.Send() }
}
