
param(
    [ValidateSet("New", "ReplyAll")][string]$Mode = "New",
    [string]$To = "", [string]$Cc = "", [string]$Subject = "",
    [string]$HtmlBodyPath = "", [string]$AttachmentListPath = "",
    [ValidateSet("Send()", "Send", "Display", "SaveDraft")][string]$DisplayOrSend = "Send()",
    [string]$BodyMode = "Replace", [string]$SenderAccount = "",
    [string]$SubjectPrefix = "", [string]$AdditionalTo = "",
    [string]$SendAtNextSevenAm = "false"
)
function Write-OutlookStep([string]$Message) { Write-Output ("TRACE: [Outlook] " + $Message) }
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
    Write-OutlookStep "Reading Outlook folder items for original-message search"
    $items = $Folder.Items
    Write-OutlookStep ("Folder item count=" + $items.Count)
    $items.Sort($SortProperty, $true)
    foreach ($item in $items) {
        if (-not $item.Subject -or -not $item.Subject.StartsWith($SubjectPrefix, [System.StringComparison]::OrdinalIgnoreCase)) { continue }
        Write-OutlookStep "Subject prefix matched; checking sender account"
        $senderAddress = Get-OutlookSenderAddress $item
        if ([string]::Equals($senderAddress.Trim(), $SenderAccount.Trim(), [System.StringComparison]::OrdinalIgnoreCase)) {
            Write-OutlookStep "Original-message sender matched"
            return $item
        }
    }
    Write-OutlookStep "No matching original message in this folder"
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
        [ValidateSet("Send()", "Send", "Display", "SaveDraft")][string]$DisplayOrSend = "Send()",
        [string]$BodyMode = "Replace", [string]$SenderAccount = "",
        [string]$SubjectPrefix = "", [string]$AdditionalTo = "",
        [switch]$SendAtNextSevenAm
    )
    $ErrorActionPreference = "Stop"
    $stage = "initialize"
    try {
    Write-OutlookStep "Starting; mode=$Mode; action=$DisplayOrSend"
    $stage = "create Outlook COM instance"
    Write-OutlookStep "Creating Outlook.Application"
    $outlook = New-Object -ComObject Outlook.Application
    Write-OutlookStep "Outlook.Application created"
    if ($Mode -eq "ReplyAll") {
        if ([string]::IsNullOrWhiteSpace($SenderAccount) -or [string]::IsNullOrWhiteSpace($SubjectPrefix)) { throw "ReplyAll requires SenderAccount and SubjectPrefix." }
        $stage = "find original message"
        Write-OutlookStep "Searching Inbox for original message"
        $namespace = $outlook.GetNamespace("MAPI")
        $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(6)) "[ReceivedTime]" $SenderAccount $SubjectPrefix
        if (-not $mail) {
            Write-OutlookStep "Inbox match not found; searching Sent Items"
            $mail = Find-OutlookOriginalMessage ($namespace.GetDefaultFolder(5)) "[SentOn]" $SenderAccount $SubjectPrefix
        }
        if (-not $mail) { throw "Original message with subject prefix '$SubjectPrefix' not found in Inbox or Sent Items of '$SenderAccount'." }
        Write-OutlookStep "Original message found"
        $stage = "create ReplyAll"
        $mail = $mail.ReplyAll()
        Write-OutlookStep "ReplyAll created"
    } else {
        $stage = "create new mail"
        Write-OutlookStep "Creating new mail item"
        $mail = $outlook.CreateItem(0)
        if ($To) { $mail.To = $To }
        if ($Cc) { $mail.CC = $Cc }
    }
    Write-OutlookStep "Mail item ready"
    $stage = "update recipients"
    if ($AdditionalTo -and $Mode -eq "ReplyAll") {
        Write-OutlookStep "Checking additional recipient"
        $currentTo = [string]$mail.To
        if ($currentTo.IndexOf($AdditionalTo, [System.StringComparison]::OrdinalIgnoreCase) -lt 0) {
            if ([string]::IsNullOrWhiteSpace($currentTo)) { $mail.To = $AdditionalTo }
            else { $mail.To = $currentTo.TrimEnd(';', ' ', ',') + "; " + $AdditionalTo }
        }
    }
    Write-OutlookStep "Recipient stage complete"
    $stage = "set subject"
    if ($Subject) { $mail.Subject = $Subject }
    Write-OutlookStep "Subject stage complete"
    $stage = "load HTML body"
    if ($HtmlBodyPath) {
        Write-OutlookStep "Reading HTML body file"
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
    Write-OutlookStep "HTML body stage complete"
    $stage = "load attachment list"
    $AttachmentPaths = @()
    if ($AttachmentListPath) {
        Write-OutlookStep "Reading attachment list"
        if (-not (Test-Path -LiteralPath $AttachmentListPath -PathType Leaf)) { throw "Attachment list file not found: $AttachmentListPath" }
        $AttachmentPaths = @(Get-Content -LiteralPath $AttachmentListPath -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    }
    Write-OutlookStep ("Attachment list loaded; count=" + $AttachmentPaths.Count)
    $stage = "add attachments"
    foreach ($attachmentPath in $AttachmentPaths) {
        Write-OutlookStep "Validating and adding attachment"
        if ([string]::IsNullOrWhiteSpace($attachmentPath)) { continue }
        if (-not (Test-Path -LiteralPath $attachmentPath -PathType Leaf)) { throw "Email attachment not found: $attachmentPath" }
        $mail.Attachments.Add([System.IO.Path]::GetFullPath($attachmentPath)) | Out-Null
    }
    Write-OutlookStep "Attachment stage complete"
    if ($SendAtNextSevenAm -eq "true" -and $DisplayOrSend -ne "SaveDraft") {
        $stage = "set deferred delivery time"
        Write-OutlookStep "Setting deferred delivery time"
        $now = Get-Date
        $targetTime = $now.Date.AddHours(7)
        if ($now -ge $targetTime) { $targetTime = $targetTime.AddDays(1) }
        $mail.DeferredDeliveryTime = $targetTime
    } elseif ($DisplayOrSend -eq "SaveDraft" -and $SendAtNextSevenAm -eq "true") {
        Write-OutlookStep "Deferred delivery skipped for draft"
    }
    $stage = "final action"
    switch ($DisplayOrSend) {
        "Display" { Write-OutlookStep "Displaying item"; $mail.Display() }
        "SaveDraft" { Write-OutlookStep "Saving draft"; $mail.Save(); Write-OutlookStep "Draft saved" }
        default { Write-OutlookStep "Sending item"; $mail.Send(); Write-OutlookStep "Send returned successfully" }
    }
    Write-OutlookStep "Operation completed"
    } catch {
        Write-OutlookStep ("FAILED at stage '" + $stage + "': " + $_.Exception.Message)
        throw
    }
}


$ErrorActionPreference = "Stop"
try {
    $attachments = @()
    if ($AttachmentListPath) {
        if (-not (Test-Path -LiteralPath $AttachmentListPath -PathType Leaf)) { throw "Attachment list file not found: $AttachmentListPath" }
        $attachments = @(Get-Content -LiteralPath $AttachmentListPath -Encoding UTF8 | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    }
    Write-OutlookStep ("Script entry; attachment count=" + $attachments.Count)
    $sendAtSeven = $SendAtNextSevenAm -eq "true"
    Send-OutlookMail -Mode $Mode -To $To -Cc $Cc -Subject $Subject -HtmlBodyPath $HtmlBodyPath -AttachmentPaths $attachments -DisplayOrSend $DisplayOrSend -BodyMode $BodyMode -SenderAccount $SenderAccount -SubjectPrefix $SubjectPrefix -AdditionalTo $AdditionalTo -SendAtNextSevenAm:$sendAtSeven
} catch {
    Write-Error ("Outlook operation failed: " + $_.Exception.Message)
    exit 1
}
