$ErrorActionPreference = "Stop"

$rootDir = "E:\mc\Allay"
$nsisDir = "$rootDir\target\release\bundle\nsis"

# --- Step 1: Build the Tauri app (NSIS installer only) ---
Write-Output "=== BUILDING ==="
Set-Location "$rootDir\apps\app"
pnpm tauri build --config tauri-release.conf.json 2>&1

# --- Step 2: Decode the base64-wrapped private key to plain text ---
# Tauri CLI uses base64-wrapped key files; rsign2 needs plain text format
Write-Output "=== DECODING KEY ==="
$keyB64 = [System.IO.File]::ReadAllText("$rootDir\apps\app\allay-updater-key", [System.Text.Encoding]::UTF8).Trim()
$keyBytes = [Convert]::FromBase64String($keyB64)
$keyPlain = [System.Text.Encoding]::UTF8.GetString($keyBytes)
$keyPlainPath = "$env:TEMP\allay-key-plain"
$keyPlain | Set-Content $keyPlainPath -NoNewline

# --- Step 3: Sign the updater bundles with rsign2 ---
Write-Output "=== SIGNING ==="
$exeFile = Get-ChildItem "$nsisDir\*_x64-setup.exe" | Select-Object -First 1
$zipFile = Get-ChildItem "$nsisDir\*_x64-setup.nsis.zip" | Select-Object -First 1

if (-not $exeFile -or -not $zipFile) {
    Write-Error "Build output files not found in $nsisDir"
    exit 1
}

rsign sign -s $keyPlainPath -W -x "$nsisDir\$($zipFile.Name).sig" "$nsisDir\$($zipFile.Name)"
rsign sign -s $keyPlainPath -W -x "$nsisDir\$($exeFile.Name).sig" "$nsisDir\$($exeFile.Name)"

# --- Step 4: Generate latest.json ---
Write-Output "=== GENERATING latest.json ==="
$sigRaw = Get-Content "$nsisDir\$($zipFile.Name).sig" -Raw
# Tauri updater expects base64-encoded signature (same as pubkey format)
$sigBytes = [System.Text.Encoding]::UTF8.GetBytes($sigRaw.Trim())
$sigB64 = [Convert]::ToBase64String($sigBytes)
$version = (Get-Content "$rootDir\apps\app-frontend\package.json" | ConvertFrom-Json).version
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$latest = @{
    version  = $version
    notes    = ""
    pub_date = $pubDate
    platforms = @{
        "windows-x86_64" = @{
            signature = $sigB64
            url       = "https://github.com/luolita13/Allay/releases/download/v$version/$($zipFile.Name)"
        }
    }
}
# Use UTF-8 without BOM — JSON parsers reject BOM
$latestJson = $latest | ConvertTo-Json -Depth 4
$Utf8NoBom = New-Object System.Text.UTF8Encoding $false
[System.IO.File]::WriteAllText("$nsisDir\latest.json", $latestJson, $Utf8NoBom)

# --- Step 5: Verify signatures ---
Write-Output "=== VERIFYING SIGNATURES ==="
$pubB64 = [System.IO.File]::ReadAllText("$rootDir\apps\app\allay-updater-key.pub", [System.Text.Encoding]::UTF8).Trim()
$pubBytes = [Convert]::FromBase64String($pubB64)
$pubPlain = [System.Text.Encoding]::UTF8.GetString($pubBytes)
$pubPlainPath = "$env:TEMP\allay-pub-plain"
$pubPlain | Set-Content $pubPlainPath -NoNewline

rsign verify -x "$nsisDir\$($zipFile.Name).sig" -p $pubPlainPath -q "$nsisDir\$($zipFile.Name)"
if ($LASTEXITCODE -ne 0) { Write-Error "ZIP signature verification failed!"; exit 1 }

rsign verify -x "$nsisDir\$($exeFile.Name).sig" -p $pubPlainPath -q "$nsisDir\$($exeFile.Name)"
if ($LASTEXITCODE -ne 0) { Write-Error "EXE signature verification failed!"; exit 1 }

# --- Cleanup ---
Remove-Item $keyPlainPath, $pubPlainPath -Force -ErrorAction SilentlyContinue

# --- Output ---
Write-Output "=== OUTPUT (5 files) ==="
Get-ChildItem $nsisDir | Where-Object { -not $_.PSIsContainer } | ForEach-Object {
    Write-Output ("  {0} ({1:N0} bytes)" -f $_.Name, $_.Length)
}
