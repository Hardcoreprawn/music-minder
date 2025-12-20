# Generate application icon for Music Minder
# Creates a multi-resolution ICO file from PNG images
# 
# Prerequisites: None - uses built-in .NET System.Drawing
# Usage: .\scripts\generate-icon.ps1

param(
    [string]$OutputPath = "assets/icon.ico"
)

Add-Type -AssemblyName System.Drawing

# Icon sizes needed for Windows (16, 32, 48, 256)
$sizes = @(16, 32, 48, 256)

# Create a musical note icon programmatically
function New-MusicIcon {
    param([int]$Size)
    
    $bitmap = New-Object System.Drawing.Bitmap($Size, $Size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    
    # High quality rendering
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    
    # Scale factor
    $scale = $Size / 256.0
    
    # Background - rounded rectangle with gradient
    $bgBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.Point(0, 0)),
        (New-Object System.Drawing.Point($Size, $Size)),
        [System.Drawing.Color]::FromArgb(255, 102, 126, 234),  # #667eea
        [System.Drawing.Color]::FromArgb(255, 118, 75, 162)   # #764ba2
    )
    
    # Draw rounded rectangle background
    $cornerRadius = [int](48 * $scale)
    $bgPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    $rect = New-Object System.Drawing.Rectangle([int](8 * $scale), [int](8 * $scale), [int](240 * $scale), [int](240 * $scale))
    
    if ($cornerRadius -gt 0) {
        $diameter = $cornerRadius * 2
        $arc = New-Object System.Drawing.Rectangle($rect.Left, $rect.Top, $diameter, $diameter)
        $bgPath.AddArc($arc, 180, 90)
        $arc.X = $rect.Right - $diameter
        $bgPath.AddArc($arc, 270, 90)
        $arc.Y = $rect.Bottom - $diameter
        $bgPath.AddArc($arc, 0, 90)
        $arc.X = $rect.Left
        $bgPath.AddArc($arc, 90, 90)
        $bgPath.CloseFigure()
    } else {
        $bgPath.AddRectangle($rect)
    }
    
    $graphics.FillPath($bgBrush, $bgPath)
    
    # White brush for notes
    $whiteBrush = [System.Drawing.Brushes]::White
    
    # Musical note ellipses (note heads)
    $graphics.FillEllipse($whiteBrush, 
        [int]((80 - 32) * $scale), [int]((180 - 28) * $scale),
        [int](64 * $scale), [int](56 * $scale))
    $graphics.FillEllipse($whiteBrush,
        [int]((176 - 32) * $scale), [int]((156 - 28) * $scale),
        [int](64 * $scale), [int](56 * $scale))
    
    # Note stems
    $graphics.FillRectangle($whiteBrush,
        [int](104 * $scale), [int](68 * $scale),
        [int](12 * $scale), [int](112 * $scale))
    $graphics.FillRectangle($whiteBrush,
        [int](200 * $scale), [int](44 * $scale),
        [int](12 * $scale), [int](112 * $scale))
    
    # Connecting beam (as polygon)
    $beamPoints = @(
        (New-Object System.Drawing.PointF([float](104 * $scale), [float](68 * $scale))),
        (New-Object System.Drawing.PointF([float](212 * $scale), [float](44 * $scale))),
        (New-Object System.Drawing.PointF([float](212 * $scale), [float](60 * $scale))),
        (New-Object System.Drawing.PointF([float](104 * $scale), [float](84 * $scale)))
    )
    $graphics.FillPolygon($whiteBrush, $beamPoints)
    
    # Cleanup
    $graphics.Dispose()
    $bgBrush.Dispose()
    $bgPath.Dispose()
    
    return $bitmap
}

# Create ICO file with multiple sizes
function New-IconFile {
    param(
        [string]$Path,
        [int[]]$Sizes
    )
    
    $iconData = New-Object System.Collections.Generic.List[byte[]]
    $bitmaps = @()
    
    foreach ($size in $Sizes) {
        Write-Host "Generating ${size}x${size} icon..."
        $bitmap = New-MusicIcon -Size $size
        $bitmaps += $bitmap
        
        # Convert to PNG bytes
        $stream = New-Object System.IO.MemoryStream
        $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
        $iconData.Add($stream.ToArray())
        $stream.Dispose()
    }
    
    # Build ICO file structure
    $icoStream = New-Object System.IO.MemoryStream
    $writer = New-Object System.IO.BinaryWriter($icoStream)
    
    # ICO header (6 bytes)
    $writer.Write([int16]0)      # Reserved
    $writer.Write([int16]1)      # Type: 1 = ICO
    $writer.Write([int16]$Sizes.Count)  # Number of images
    
    # Calculate offsets
    $headerSize = 6
    $entrySize = 16
    $dataOffset = $headerSize + ($entrySize * $Sizes.Count)
    
    # Write directory entries
    for ($i = 0; $i -lt $Sizes.Count; $i++) {
        $size = $Sizes[$i]
        $data = $iconData[$i]
        
        # Width/Height (0 means 256)
        $writer.Write([byte]$(if ($size -eq 256) { 0 } else { $size }))
        $writer.Write([byte]$(if ($size -eq 256) { 0 } else { $size }))
        $writer.Write([byte]0)     # Color palette
        $writer.Write([byte]0)     # Reserved
        $writer.Write([int16]1)    # Color planes
        $writer.Write([int16]32)   # Bits per pixel
        $writer.Write([int32]$data.Length)  # Image size
        $writer.Write([int32]$dataOffset)   # Offset to image data
        
        $dataOffset += $data.Length
    }
    
    # Write image data
    foreach ($data in $iconData) {
        $writer.Write($data)
    }
    
    # Save to file
    $writer.Flush()
    [System.IO.File]::WriteAllBytes($Path, $icoStream.ToArray())
    
    # Cleanup
    $writer.Dispose()
    $icoStream.Dispose()
    foreach ($bitmap in $bitmaps) {
        $bitmap.Dispose()
    }
    
    Write-Host "Icon saved to: $Path"
}

# Run
$fullPath = Join-Path $PSScriptRoot "..\$OutputPath"
$fullPath = [System.IO.Path]::GetFullPath($fullPath)
New-IconFile -Path $fullPath -Sizes $sizes
Write-Host "Done!"
