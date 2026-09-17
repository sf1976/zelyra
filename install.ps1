param(
    [string]$InstallRoot = (Join-Path $env:LOCALAPPDATA "Zelyra")
)

$ErrorActionPreference = "Stop"

$scriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$cargo = Get-Command cargo -ErrorAction SilentlyContinue

if ($null -eq $cargo) {
    $rustup = Join-Path $env:TEMP "zelyra-rustup-init.exe"
    Write-Host "Rust is missing; installing the user-local stable toolchain..."
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y --profile minimal
    Remove-Item -LiteralPath $rustup -Force

    $cargoDirectory = Join-Path $env:USERPROFILE ".cargo\bin"
    $env:Path = "$cargoDirectory;$env:Path"
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
}

if ($null -eq $cargo) {
    throw "cargo is still unavailable after Rust installation. Open a new PowerShell and run install.ps1 again."
}

Write-Host "Building and installing Zelyra for the current user..."
& $cargo.Source install --path (Join-Path $scriptDirectory "cli") --root $InstallRoot --force

$binDirectory = Join-Path $InstallRoot "bin"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @()
if (-not [string]::IsNullOrWhiteSpace($userPath)) {
    $pathEntries = $userPath -split ";" | Where-Object { $_ }
}
if ($pathEntries -notcontains $binDirectory) {
    $updatedPath = (($pathEntries + $binDirectory) -join ";")
    [Environment]::SetEnvironmentVariable("Path", $updatedPath, "User")
    $env:Path = "$binDirectory;$env:Path"
}

Write-Host ""
Write-Host "Zelyra installed to $(Join-Path $binDirectory "zelyra.exe")"
Write-Host "Open a new terminal if the zelyra command is not available yet."
Write-Host "Try: zelyra run $(Join-Path $scriptDirectory "examples\fibonacci.zyl")"
