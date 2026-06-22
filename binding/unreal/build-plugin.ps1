param(
    [Parameter(Mandatory = $true)]
    [string]$UE,
    [string]$Output = "",
    [switch]$Full
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
if ($Output -eq "") {
    $Output = Join-Path $ScriptDir "build\DMVOPBridge"
}

$PluginDir = Join-Path $ScriptDir "DMVOPBridge"
$Plugin = Join-Path $PluginDir "DMVOPBridge.uplugin"
$HostDir = Join-Path $ScriptDir ".host"
$HostPluginDir = Join-Path $HostDir "Plugins\DMVOPBridge"
$HostProj = Join-Path $HostDir "HostProject.uproject"
$UAT = Join-Path $UE "Engine\Build\BatchFiles\RunUAT.bat"
$UBT = Join-Path $UE "Engine\Build\BatchFiles\Build.bat"

if (-not (Test-Path $UAT)) {
    Write-Host "UE not found at $UE" -ForegroundColor Red
    exit 1
}

# ── Ensure .host project + plugin symlink ──
if (-not (Test-Path $HostPluginDir)) {
    New-Item -ItemType Directory -Path (Join-Path $HostDir "Plugins") -Force | Out-Null
    cmd /c "mklink /J `"$HostPluginDir`" `"$PluginDir`"" | Out-Null
}

if (-not (Test-Path $HostProj)) {
    $proj = @"
{
    "FileVersion": 3,
    "EngineAssociation": "5.6",
    "Category": "",
    "Description": "",
    "Plugins": [
        { "Name": "DMVOPBridge", "Enabled": true }
    ]
}
"@
    Set-Content -Path $HostProj -Value $proj
}

if ($Full -or -not (Test-Path $Output)) {
    # ── Full build ──
    Write-Host "=== DMVOPBridge (full) ===" -ForegroundColor Cyan
    & $UAT BuildPlugin -Plugin="$Plugin" -Package="$Output" -Rocket
} else {
    # ── Incremental build ──
    Write-Host "=== DMVOPBridge (incremental) ===" -ForegroundColor Cyan
    & $UBT UnrealEditor Win64 Development `
        -Project="$HostProj" `
        -Plugin="$Plugin" `
        -TargetType=Editor

    # Ensure output has the base plugin files (.uplugin, Config, etc.)
    if (-not (Test-Path (Join-Path $Output "DMVOPBridge.uplugin"))) {
        Write-Host "  Copying plugin skeleton..." -ForegroundColor DarkYellow
        Copy-Item (Join-Path $PluginDir "*.uplugin") $Output -Force
        if (Test-Path (Join-Path $PluginDir "Config")) {
            Copy-Item (Join-Path $PluginDir "Config") $Output -Recurse -Force
        }
    }

    # Copy compiled binaries
    $DllSrc = "$PluginDir\Binaries"
    $DllDst = "$Output\Binaries"
    if (Test-Path $DllSrc) {
        Copy-Item $DllSrc $DllDst -Recurse -Force
        Write-Host "  Updated: $Output\Binaries"
    }
}

if ($LASTEXITCODE -eq 0) {
    Write-Host "=== Done ===" -ForegroundColor Green
} else {
    Write-Host "Build failed" -ForegroundColor Red
}
