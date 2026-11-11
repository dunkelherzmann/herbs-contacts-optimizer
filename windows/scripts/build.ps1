# Builds Herb's Contacts Optimizer for Windows: one static, portable .exe.
#   powershell -File windows\scripts\build.ps1 [-NoDesktop] [-Version 0.1.25]
# Puts it in dist\ and, unless -NoDesktop, on the Desktop, then runs its --self-check.
# Needs Rust (rustup, per user) and the Visual Studio Build Tools. -Version is for a copy of the files
# without git history; otherwise the version is counted as scripts/version.sh counts it.
param([switch]$NoDesktop, [string]$Version = '')
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not $Version) {
    $base = (Get-Content (Join-Path $root 'VERSION') -Raw).Trim()
    $since = git -C $root log -1 --format=%H -- VERSION 2>$null
    $patch = if ($since) { (git -C $root rev-list --count "$since..HEAD").Trim() } else { '0' }
    $Version = "$base.$patch"
}
Set-Content -Path (Join-Path $root 'version.txt') -Value $Version -NoNewline
Push-Location $root
try {
    cargo build --release -p herbs-contacts-optimizer
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally { Pop-Location }
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root 'target' }
$exe = Join-Path $target 'release\herbs-contacts-optimizer.exe'
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force $dist | Out-Null
$out = Join-Path $dist 'HerbsContactsOptimizer.exe'
Copy-Item $exe $out -Force
# A windowed program prints nothing to this console; its exit code tells.
$check = Start-Process -FilePath $out -ArgumentList '--self-check' -Wait -PassThru
if ($check.ExitCode -ne 0) { Write-Error 'build.ps1: the self-check failed'; exit 1 }
if (-not $NoDesktop) {
    Copy-Item $out (Join-Path ([Environment]::GetFolderPath('Desktop')) 'Herbs Contacts Optimizer.exe') -Force
    Write-Host 'Copied to the Desktop'
}
Write-Host "Built dist\HerbsContactsOptimizer.exe $Version ($((Get-Item $out).Length / 1MB -as [int]) MB), self-check passed"
