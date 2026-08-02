$ErrorActionPreference = "Stop"

$rootDir = "E:\mc\Allay"
$nsisDir = "$rootDir\target\release\bundle\nsis"

# --- Step 0: Clean old artifacts so we never sign/resurrect stale files ---
Write-Output "=== CLEANING OLD NSIS ARTIFACTS ==="
if (Test-Path $nsisDir) {
    [System.IO.Directory]::GetFiles($nsisDir, "Allay_*") | ForEach-Object {
        [System.IO.File]::Delete($_)
        Write-Output "  deleted $_"
    }
}

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

# --- Step 3: Locate this release's files by exact version ---
Write-Output "=== LOCATING BUNDLE FILES ==="
$version = (Get-Content "$rootDir\apps\app-frontend\package.json" | ConvertFrom-Json).version
$expectedExe = "Allay_${version}_x64-setup.exe"
$expectedZip = "Allay_${version}_x64-setup.nsis.zip"
$exeFile = Get-Item "$nsisDir\$expectedExe" -ErrorAction SilentlyContinue
$zipFile = Get-Item "$nsisDir\$expectedZip" -ErrorAction SilentlyContinue

if (-not $exeFile -or -not $zipFile) {
    Write-Error "Expected bundle files for version $version not found in $nsisDir"
    Write-Output "Expected: $expectedExe, $expectedZip"
    exit 1
}
Write-Output "  exe: $($exeFile.Name)"
Write-Output "  zip: $($zipFile.Name)"

# --- Step 4: Sign the updater bundles with rsign2 ---
Write-Output "=== SIGNING ==="

rsign sign -s $keyPlainPath -W -x "$nsisDir\$($zipFile.Name).sig" "$nsisDir\$($zipFile.Name)"
rsign sign -s $keyPlainPath -W -x "$nsisDir\$($exeFile.Name).sig" "$nsisDir\$($exeFile.Name)"

# --- Step 5: Generate latest.json ---
Write-Output "=== GENERATING latest.json ==="
$sigRaw = [System.IO.File]::ReadAllText("$nsisDir\$($zipFile.Name).sig", [System.Text.Encoding]::UTF8)
# Tauri updater expects base64-encoded signature (same as pubkey format).
# Do NOT trim the trailing newline — rsign signatures require it.
$sigBytes = [System.Text.Encoding]::UTF8.GetBytes($sigRaw)
$sigB64 = [Convert]::ToBase64String($sigBytes)
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

# Sanity check: ensure latest.json version matches the release we just built
$generated = Get-Content "$nsisDir\latest.json" -Raw | ConvertFrom-Json
if ($generated.version -ne $version) {
    Write-Error "latest.json version mismatch: generated '$($generated.version)' != expected '$version'"
    exit 1
}
Write-Output "  latest.json version: $($generated.version)"

# --- Step 6: Verify signatures ---
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
