# Generate brand assets: favicon.ico / apple-touch-icon.png / og-image.png (ASCII-only)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$root = 'D:\工作2026\TopSSL\SSL-Client\demo1\site\public'
New-Item -ItemType Directory -Force -Path $root | Out-Null

$c1 = [System.Drawing.Color]::FromArgb(255, 60, 197, 226)
$c2 = [System.Drawing.Color]::FromArgb(255, 0, 133, 161)
$c3 = [System.Drawing.Color]::FromArgb(255, 3, 107, 129)
$white = [System.Drawing.Color]::White

function New-RoundedPath([float]$x, [float]$y, [float]$w, [float]$h, [float]$r) {
  $p = New-Object System.Drawing.Drawing2D.GraphicsPath
  $p.AddArc($x, $y, $r * 2, $r * 2, 180, 90)
  $p.AddArc($x + $w - $r * 2, $y, $r * 2, $r * 2, 270, 90)
  $p.AddArc($x + $w - $r * 2, $y + $h - $r * 2, $r * 2, $r * 2, 0, 90)
  $p.AddArc($x, $y + $h - $r * 2, $r * 2, $r * 2, 90, 90)
  $p.CloseFigure()
  return $p
}

function New-BrandIcon([int]$size) {
  $bmp = New-Object System.Drawing.Bitmap($size, $size)
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::Transparent)
  $rect = New-Object System.Drawing.RectangleF(0, 0, $size, $size)
  $grad = New-Object System.Drawing.Drawing2D.LinearGradientBrush($rect, $c1, $c3, 135)
  $path = New-RoundedPath 0 0 $size $size ([float]($size * 0.22))
  $g.FillPath($grad, $path)

  $penW = [math]::Max(1.2, $size * 0.075)
  $pen = New-Object System.Drawing.Pen($white, $penW)
  $pen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
  $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round

  # shackle (upper arc)
  $sw = $size * 0.32
  $sx = ($size - $sw) / 2
  $sy = $size * 0.18
  $g.DrawArc($pen, $sx, $sy, $sw, $sw, 180, 180)

  # body (rounded rect outline)
  $bx = $size * 0.19
  $by = $size * 0.42
  $bw = $size * 0.62
  $bh = $size * 0.4
  $br = $size * 0.07
  $body = New-RoundedPath $bx $by $bw $bh $br
  $g.DrawPath($pen, $body)

  # keyhole
  $kw = $size * 0.09
  $kx = ($size - $kw) / 2
  $ky = $size * 0.58
  $g.DrawEllipse($pen, $kx, $ky, $kw, $kw)

  $g.Dispose()
  return $bmp
}

# favicon.ico with PNG-compressed entries
$sizes = @(16, 32, 48, 64, 128)
$blobs = @()
foreach ($s in $sizes) {
  $bmp = New-BrandIcon $s
  $ms = New-Object System.IO.MemoryStream
  $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
  $blobs += ,@($s, $ms.ToArray())
  $ms.Dispose()
  $bmp.Dispose()
}
$header = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($header)
$bw.Write([UInt16]0); $bw.Write([UInt16]1); $bw.Write([UInt16]$blobs.Count)
$offset = 6 + 16 * $blobs.Count
foreach ($b in $blobs) {
  $s = $b[0]; $data = $b[1]
  $bw.Write([Byte]($(if ($s -ge 256) { 0 } else { $s })))
  $bw.Write([Byte]($(if ($s -ge 256) { 0 } else { $s })))
  $bw.Write([Byte]0); $bw.Write([Byte]0)
  $bw.Write([UInt16]1); $bw.Write([UInt16]32)
  $bw.Write([UInt32]$data.Length); $bw.Write([UInt32]$offset)
  $offset += $data.Length
}
foreach ($b in $blobs) { $bw.Write($b[1]) }
$bw.Flush()
[System.IO.File]::WriteAllBytes("$root\favicon.ico", $header.ToArray())
$bw.Dispose(); $header.Dispose()

# apple-touch-icon.png
$icon = New-BrandIcon 180
$icon.Save("$root\apple-touch-icon.png", [System.Drawing.Imaging.ImageFormat]::Png)
$icon.Dispose()

# og-image.png 1200x630
$W = 1200; $H = 630
$og = New-Object System.Drawing.Bitmap($W, $H)
$g2 = [System.Drawing.Graphics]::FromImage($og)
$g2.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g2.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

$bgRect = New-Object System.Drawing.RectangleF(0, 0, $W, $H)
$bg = New-Object System.Drawing.Drawing2D.LinearGradientBrush($bgRect, [System.Drawing.Color]::FromArgb(255, 4, 39, 47), [System.Drawing.Color]::FromArgb(255, 3, 32, 42), 90)
$g2.FillRectangle($bg, $bgRect)
$glow = New-Object System.Drawing.Drawing2D.GraphicsPath
$glow.AddEllipse(-200, -260, 900, 700)
$gb = New-Object System.Drawing.Drawing2D.PathGradientBrush($glow)
$gb.CenterColor = [System.Drawing.Color]::FromArgb(110, 0, 133, 161)
$gb.SurroundColors = @([System.Drawing.Color]::FromArgb(0, 0, 133, 161))
$g2.FillPath($gb, $glow)

$mini = New-BrandIcon 64
$g2.DrawImage($mini, 72, 64, 64, 64)
$mini.Dispose()
$fName = New-Object System.Drawing.Font('Microsoft YaHei', 26, [System.Drawing.FontStyle]::Bold)
$bWhite = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 231, 246, 249))
$g2.DrawString('TopSSL 免费证书助手', $fName, $bWhite, 150, 82)

$fH1 = New-Object System.Drawing.Font('Microsoft YaHei', 58, [System.Drawing.FontStyle]::Bold)
$g2.DrawString('免费 SSL 证书，一键申请，自动续期', $fH1, $bWhite, 72, 210)

$fSub = New-Object System.Drawing.Font('Microsoft YaHei', 24)
$bSub = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 169, 198, 207))
$g2.DrawString('支持 HTTP / DNS 验证 · 通配符证书 · Windows / macOS / Linux · MIT 开源', $fSub, $bSub, 72, 330)

$fFoot = New-Object System.Drawing.Font('Microsoft YaHei', 18)
$bFoot = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(255, 124, 160, 176))
$g2.DrawString('由 TopSSL 出品并支持 · www.topssl.cn', $fFoot, $bFoot, 72, 540)

$og.Save("$root\og-image.png", [System.Drawing.Imaging.ImageFormat]::Png)
$og.Dispose()
$g2.Dispose()

Write-Output 'ASSETS OK'
Get-ChildItem $root | ForEach-Object { Write-Output ($_.Name + ' ' + $_.Length) }
