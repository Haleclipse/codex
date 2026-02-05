<#
  Ensures `lld-link.exe` is available for faster linking on Windows/MSVC.

  This repo configures Cargo to use `lld-link` via `codex-rs/.cargo/config.toml`.

  If LLVM isn't installed, this script will copy Rust's bundled `rust-lld.exe`
  to `~/.cargo/bin/lld-link.exe` so Cargo can find it.

  Usage (from codex-rs):
    powershell -ExecutionPolicy Bypass -File scripts/setup-lld-link.ps1
#>

$ErrorActionPreference = 'Stop'

$existing = Get-Command lld-link -ErrorAction SilentlyContinue
if ($null -ne $existing) {
  Write-Host "lld-link already available at: $($existing.Source)" -ForegroundColor Green
  exit 0
}

$sysroot = (rustc --print sysroot)
$triple = (rustc -vV | Select-String '^host: ' | ForEach-Object { $_.Line.Split(' ')[1] })
$rustLld = Join-Path $sysroot "lib\\rustlib\\$triple\\bin\\rust-lld.exe"
if (-not (Test-Path $rustLld)) {
  throw "rust-lld not found at: $rustLld"
}

$cargoBin = Join-Path $env:USERPROFILE ".cargo\\bin"
New-Item -ItemType Directory -Force -Path $cargoBin | Out-Null

$dest = Join-Path $cargoBin "lld-link.exe"
Copy-Item -Force $rustLld $dest

if (-not ($env:Path.Split(';') -contains $cargoBin)) {
  $env:Path = "$env:Path;$cargoBin"
}

Write-Host "Installed lld-link at: $dest" -ForegroundColor Green
Write-Host "Tip: restart your terminal if Cargo still can't find it." -ForegroundColor DarkCyan

