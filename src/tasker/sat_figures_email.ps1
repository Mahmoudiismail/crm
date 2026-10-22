
param(
    [string]$CsvPath,
    [string]$ImagePath
)
$ErrorActionPreference = "Stop"
$bitmap = $null
$graphics = $null
$headerFont = $null
$valueFont = $null
$brush = $null
$headerBrush = $null
$borderPen = $null
try {
    $stage = "Load System.Drawing"
    Write-Output "TRACE: Sat Figures stage: $stage"
    Add-Type -AssemblyName System.Drawing
    $row = @(Import-Csv -LiteralPath $CsvPath)
    if ($row.Count -ne 1) { throw "Sat Figures CSV must contain exactly one summary row." }

    $columns = @("Slots Booked", "Total Slots", "Vacant Slots", "%")
    $widths = @(240, 240, 240, 240)
    [int]$width = ($widths | Measure-Object -Sum).Sum
    [int]$height = 118
    $stage = "Create Bitmap (${width}x${height})"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $bitmap = [System.Drawing.Bitmap]::new([int]$width, [int]$height)
    $stage = "Create Graphics from Bitmap"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::White)
    $stage = "Create header Font"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $headerFont = [System.Drawing.Font]::new("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
    $stage = "Create value Font"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $valueFont = [System.Drawing.Font]::new("Segoe UI", 18, [System.Drawing.FontStyle]::Regular)
    $stage = "Create drawing brushes and pen"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(35, 45, 55))
    $headerBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(46, 92, 135))
    $whiteBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $borderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::FromArgb(190, 200, 210), 1)

    $stage = "Draw Sat Figures cells"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $x = 0
    for ($i = 0; $i -lt $columns.Count; $i++) {
        $cellWidth = $widths[$i]
        $graphics.FillRectangle($headerBrush, $x, 0, $cellWidth, 48)
        $graphics.DrawRectangle($borderPen, $x, 0, $cellWidth, $height - 1)
        $headerRect = [System.Drawing.RectangleF]::new($x + 6, 0, $cellWidth - 12, 48)
        $valueRect = [System.Drawing.RectangleF]::new($x + 6, 49, $cellWidth - 12, 68)
        $format = [System.Drawing.StringFormat]::new()
        $format.Alignment = [System.Drawing.StringAlignment]::Center
        $format.LineAlignment = [System.Drawing.StringAlignment]::Center
        $graphics.DrawString($columns[$i], $headerFont, $whiteBrush, $headerRect, $format)

        $value = [string]$row[0].PSObject.Properties.Item($columns[$i]).Value
        if ($columns[$i] -eq "%") {
            $number = 0.0
            if (-not [double]::TryParse($value, [Globalization.NumberStyles]::Any, [Globalization.CultureInfo]::InvariantCulture, [ref]$number)) {
                throw "Invalid percentage value in Sat Figures CSV: $value"
            }
            $value = $number.ToString("P2", [Globalization.CultureInfo]::InvariantCulture)
        }
        $graphics.DrawString($value, $valueFont, $brush, $valueRect, $format)
        $format.Dispose()
        $x += $cellWidth
    }

    $stage = "Save PNG to $ImagePath"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $bitmap.Save($ImagePath, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Output "TRACE: Sat Figures PNG generated at $ImagePath."
} catch {
    Write-Error "Sat Figures image generation failed during stage [$stage]: $_"
    if ($_.ScriptStackTrace) { Write-Error "PowerShell stack trace: $($_.ScriptStackTrace)" }
    exit 1
} finally {
    if ($graphics) { $graphics.Dispose() }
    if ($bitmap) { $bitmap.Dispose() }
    if ($headerFont) { $headerFont.Dispose() }
    if ($valueFont) { $valueFont.Dispose() }
    if ($brush) { $brush.Dispose() }
    if ($headerBrush) { $headerBrush.Dispose() }
    if ($whiteBrush) { $whiteBrush.Dispose() }
    if ($borderPen) { $borderPen.Dispose() }
}
