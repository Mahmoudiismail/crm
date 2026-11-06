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
$whiteBrush = $null
$borderPen = $null
try {
    $stage = "Load System.Drawing"
    Write-Output "TRACE: Sat Figures stage: $stage"
    Add-Type -AssemblyName System.Drawing
    $rows = @(Import-Csv -LiteralPath $CsvPath)
    if ($rows.Count -lt 1) { throw "Sat Figures CSV must contain at least one data row." }

    $columns = @("Date", "Slots Booked", "Total Slots", "Vacant Slots", "%")
    $widths = @(190, 210, 210, 210, 170)
    [int]$width = ($widths | Measure-Object -Sum).Sum
    [int]$headerHeight = 48
    [int]$rowHeight = 64
    [int]$height = $headerHeight + ($rows.Count * $rowHeight)
    $stage = "Create Bitmap (${width}x${height})"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $bitmap = [System.Drawing.Bitmap]::new([int]$width, [int]$height)
    $stage = "Create Graphics from Bitmap"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $graphics.Clear([System.Drawing.Color]::White)
    $stage = "Create drawing fonts, brushes and pen"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $headerFont = [System.Drawing.Font]::new("Segoe UI", 12, [System.Drawing.FontStyle]::Bold)
    $valueFont = [System.Drawing.Font]::new("Segoe UI", 16, [System.Drawing.FontStyle]::Regular)
    $brush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(35, 45, 55))
    $headerBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::FromArgb(76, 175, 80))
    $whiteBrush = [System.Drawing.SolidBrush]::new([System.Drawing.Color]::White)
    $borderPen = [System.Drawing.Pen]::new([System.Drawing.Color]::Black, 1)
    $format = [System.Drawing.StringFormat]::new()
    $format.Alignment = [System.Drawing.StringAlignment]::Center
    $format.LineAlignment = [System.Drawing.StringAlignment]::Center

    $stage = "Draw Sat Figures table header"
    Write-Output "TRACE: Sat Figures stage: $stage"
    $x = 0
    for ($i = 0; $i -lt $columns.Count; $i++) {
        $cellWidth = $widths[$i]
        $graphics.FillRectangle($headerBrush, $x, 0, $cellWidth, $headerHeight)
        $graphics.DrawRectangle($borderPen, $x, 0, $cellWidth, $height - 1)
        $headerRect = [System.Drawing.RectangleF]::new($x + 5, 0, $cellWidth - 10, $headerHeight)
        $graphics.DrawString($columns[$i], $headerFont, $whiteBrush, $headerRect, $format)
        $x += $cellWidth
    }

    $stage = "Draw Sat Figures data rows"
    Write-Output "TRACE: Sat Figures stage: $stage"
    for ($rowIndex = 0; $rowIndex -lt $rows.Count; $rowIndex++) {
        $row = $rows[$rowIndex]
        $y = $headerHeight + ($rowIndex * $rowHeight)
        $x = 0
        for ($columnIndex = 0; $columnIndex -lt $columns.Count; $columnIndex++) {
            $cellWidth = $widths[$columnIndex]
            $graphics.DrawRectangle($borderPen, $x, $y, $cellWidth, $rowHeight)
            $value = [string]$row.PSObject.Properties.Item($columns[$columnIndex]).Value
            if ($columns[$columnIndex] -eq "%") {
                $number = 0.0
                if (-not [double]::TryParse($value, [Globalization.NumberStyles]::Any, [Globalization.CultureInfo]::InvariantCulture, [ref]$number)) {
                    throw "Invalid percentage value in Sat Figures CSV: $value"
                }
                $value = $number.ToString("P2", [Globalization.CultureInfo]::InvariantCulture)
            }
            $valueRect = [System.Drawing.RectangleF]::new($x + 5, $y, $cellWidth - 10, $rowHeight)
            $graphics.DrawString($value, $valueFont, $brush, $valueRect, $format)
            $x += $cellWidth
        }
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
    if ($format) { $format.Dispose() }
    if ($graphics) { $graphics.Dispose() }
    if ($bitmap) { $bitmap.Dispose() }
    if ($headerFont) { $headerFont.Dispose() }
    if ($valueFont) { $valueFont.Dispose() }
    if ($brush) { $brush.Dispose() }
    if ($headerBrush) { $headerBrush.Dispose() }
    if ($whiteBrush) { $whiteBrush.Dispose() }
    if ($borderPen) { $borderPen.Dispose() }
}
