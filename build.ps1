# dmvop build script
# Safe to run from anywhere — navigates to script dir and restores on exit.

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$OrigDir = Get-Location

# Restore original directory on any exit
trap { Set-Location $OrigDir; break }

Set-Location $ScriptDir
Write-Host "=== Building dmvop ===" -ForegroundColor Cyan

$BuildDir = Join-Path (Get-Location) "build"
$ReleaseDir = Join-Path (Get-Location) "target\release"

cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    Set-Location $OrigDir
    exit 1
}

# Create build directory
if (-not (Test-Path $BuildDir)) {
    New-Item -ItemType Directory -Path $BuildDir | Out-Null
}

Write-Host "=== Copying files ===" -ForegroundColor Cyan

# 1. dmvop.exe
Copy-Item (Join-Path $ReleaseDir "dmvop.exe") (Join-Path $BuildDir "dmvop.exe") -Force
Write-Host "  [OK] dmvop.exe"

# 2. CUDA backend
$CudaSrc = Join-Path $ReleaseDir "cuda"
$CudaDst = Join-Path $BuildDir "cuda"
if (Test-Path $CudaSrc) {
    if (-not (Test-Path $CudaDst)) { New-Item -ItemType Directory -Path $CudaDst | Out-Null }
    Copy-Item (Join-Path $CudaSrc "*") $CudaDst -Force
    Write-Host "  [OK] cuda\"
}

# 3. CPU backend
$CpuSrc = Join-Path $ReleaseDir "cpu"
$CpuDst = Join-Path $BuildDir "cpu"
if (Test-Path $CpuSrc) {
    if (-not (Test-Path $CpuDst)) { New-Item -ItemType Directory -Path $CpuDst | Out-Null }
    Copy-Item (Join-Path $CpuSrc "*") $CpuDst -Force
    Write-Host "  [OK] cpu\"
}

# 4. dmvop.toml
$TomlSrc = Join-Path (Get-Location) "dmvop.toml"
if (Test-Path $TomlSrc) {
    Copy-Item $TomlSrc (Join-Path $BuildDir "dmvop.toml") -Force
    Write-Host "  [OK] dmvop.toml"
}

# 5. LICENSE
$LicenseSrc = Join-Path (Get-Location) "LICENSE"
if (Test-Path $LicenseSrc) {
    Copy-Item $LicenseSrc (Join-Path $BuildDir "LICENSE") -Force
    Write-Host "  [OK] LICENSE"
}

# 6. README.md
$ReadmeSrc = Join-Path (Get-Location) "README.md"
if (Test-Path $ReadmeSrc) {
    Copy-Item $ReadmeSrc (Join-Path $BuildDir "README.md") -Force
    Write-Host "  [OK] README.md"
}

Write-Host "=== Done! Build output in: $BuildDir ===" -ForegroundColor Green
Write-Host ""
Write-Host "Contents:" -ForegroundColor Cyan
Get-ChildItem $BuildDir -Recurse | ForEach-Object { "  $($_.Name)" }

# Restore original directory
Set-Location $OrigDir
