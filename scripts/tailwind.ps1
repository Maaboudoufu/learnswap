<#
.SYNOPSIS
    Builds assets/css/app.css with the Tailwind standalone CLI.

.DESCRIPTION
    The standalone CLI is a single binary with no Node.js dependency. This
    script downloads the pinned version into .tailwind\ on first use (that
    directory is git-ignored) and then runs it.

.EXAMPLE
    .\scripts\tailwind.ps1
    Build once, minified.

.EXAMPLE
    .\scripts\tailwind.ps1 watch
    Rebuild on every template change. Leave it running in a second terminal.
#>
[CmdletBinding()]
param(
    [ValidateSet('build', 'watch')]
    [string]$Command = 'build'
)

$ErrorActionPreference = 'Stop'

$TailwindVersion = 'v4.3.3'
$root = Split-Path -Parent $PSScriptRoot
$binDir = Join-Path $root '.tailwind'

$arch = if ([Environment]::Is64BitOperatingSystem -and $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'arm64' } else { 'x64' }
$asset = "tailwindcss-windows-$arch.exe"
$bin = Join-Path $binDir "tailwindcss-windows-$arch-$TailwindVersion.exe"

if (-not (Test-Path $bin)) {
    Write-Host "Downloading Tailwind CLI $TailwindVersion (windows-$arch)..."
    if (-not (Test-Path $binDir)) { New-Item -ItemType Directory -Path $binDir | Out-Null }
    $url = "https://github.com/tailwindlabs/tailwindcss/releases/download/$TailwindVersion/$asset"
    $tmp = "$bin.tmp"
    Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing
    Move-Item -Path $tmp -Destination $bin -Force
}

Push-Location $root
try {
    if ($Command -eq 'watch') {
        & $bin -i ./assets/css/input.css -o ./assets/css/app.css --watch
    }
    else {
        & $bin -i ./assets/css/input.css -o ./assets/css/app.css --minify
    }
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
