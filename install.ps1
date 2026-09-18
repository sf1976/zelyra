[CmdletBinding()]
param(
    [string]$InstallRoot = "",
    [switch]$NoRustup,
    [switch]$NoPath,
    [switch]$DryRun,
    [switch]$Check,
    [switch]$Uninstall,
    [switch]$Help
)

$ErrorActionPreference = "Stop"
$InstallerVersion = "1"
$ScriptDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path

function Show-Usage {
    @"
Zelyra source installer

Usage:
  .\install.ps1 [-InstallRoot PATH] [-NoRustup] [-NoPath] [-DryRun]
  .\install.ps1 -Check [-InstallRoot PATH]
  .\install.ps1 -Uninstall [-InstallRoot PATH]

Builds the CLI from this checkout and installs it for the current user.
No administrator privileges are used.

Options:
  -InstallRoot PATH  Install below PATH (default: %LOCALAPPDATA%\Zelyra)
  -NoRustup           Fail instead of installing Rust when cargo is missing
  -NoPath             Do not update the user PATH
  -DryRun             Show actions without changing the system
  -Check              Verify an existing installation only
  -Uninstall          Remove only the installed Zelyra executable
  -Help               Show this help

Environment:
  ZELYRA_INSTALL_ROOT  Same as -InstallRoot PATH when omitted
"@
}

function Fail([string]$Message) {
    throw "Zelyra installer: $Message"
}

function Find-Cargo {
    $candidate = Get-Command cargo -ErrorAction SilentlyContinue
    if ($null -eq $candidate) {
        return $null
    }
    $candidatePath = $candidate.Source
    if ([string]::IsNullOrWhiteSpace($candidatePath)) {
        $candidatePath = $candidate.Path
    }
    if ([string]::IsNullOrWhiteSpace($candidatePath) -or
        -not (Test-Path -LiteralPath $candidatePath -PathType Leaf)) {
        return $null
    }
    return $candidate
}

if ($Help) {
    Show-Usage
    exit 0
}

if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
    $InstallRoot = $env:ZELYRA_INSTALL_ROOT
}
if ([string]::IsNullOrWhiteSpace($InstallRoot)) {
    $InstallRoot = Join-Path $env:LOCALAPPDATA "Zelyra"
}
$InstallRoot = [IO.Path]::GetFullPath($InstallRoot)
$BinaryDirectory = Join-Path $InstallRoot "bin"
$BinaryPath = Join-Path $BinaryDirectory "zelyra.exe"

if ($Check -and $Uninstall) {
    Fail "-Check and -Uninstall cannot be combined"
}
if (-not (Test-Path -LiteralPath (Join-Path $ScriptDirectory "Cargo.toml") -PathType Leaf)) {
    Fail "Cargo.toml not found in $ScriptDirectory"
}
if (-not (Test-Path -LiteralPath (Join-Path $ScriptDirectory "cli\Cargo.toml") -PathType Leaf)) {
    Fail "CLI package not found in $ScriptDirectory\cli"
}

function Remove-Installation {
    if (-not (Test-Path -LiteralPath $BinaryPath)) {
        Write-Host "Already absent: $BinaryPath"
        return
    }
    if ((Get-Item -LiteralPath $BinaryPath).PSIsContainer) {
        Fail "Refusing to remove non-file target: $BinaryPath"
    }
    if ($DryRun) {
        Write-Host "+ Remove-Item -LiteralPath '$BinaryPath'"
    } else {
        Remove-Item -LiteralPath $BinaryPath -Force
        Write-Host "Removed: $BinaryPath"
    }
}

function Test-Installation {
    if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
        Fail "Zelyra is not installed at $BinaryPath"
    }
    & $BinaryPath --help *> $null
    if ($LASTEXITCODE -ne 0) {
        Fail "installed executable did not pass --help"
    }
    Write-Host "OK: $BinaryPath"
}

if ($Uninstall) {
    Remove-Installation
    exit 0
}
if ($Check) {
    Test-Installation
    exit 0
}

Write-Host "Zelyra installer $InstallerVersion"
Write-Host "Source: $ScriptDirectory"
Write-Host "Target: $BinaryPath"
if ($DryRun) {
    Write-Host "Mode:   dry-run"
}

$cargo = Find-Cargo
if ($null -eq $cargo) {
    if ($NoRustup) {
        Fail "cargo is missing; install Rust or omit -NoRustup"
    }
    if ($DryRun) {
        Write-Host "+ install user-local Rust with rustup"
    } else {
        $rustup = Join-Path $env:TEMP "zelyra-rustup-$PID.exe"
        try {
            Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
            & $rustup -y --profile minimal
            if ($LASTEXITCODE -ne 0) {
                Fail "Rust installation failed with exit code $LASTEXITCODE"
            }
        } finally {
            if (Test-Path -LiteralPath $rustup) {
                Remove-Item -LiteralPath $rustup -Force -ErrorAction SilentlyContinue
            }
        }
        $cargoDirectory = Join-Path $env:USERPROFILE ".cargo\bin"
        $env:Path = "$cargoDirectory;$env:Path"
        $cargo = Find-Cargo
        if ($null -eq $cargo) {
            Fail "cargo is still unavailable after Rust installation"
        }
    }
}

if ($DryRun) {
    Write-Host "+ cargo install --locked --path '$(Join-Path $ScriptDirectory "cli")' --root '$InstallRoot' --force"
} else {
    & $cargo.Source install --locked --path (Join-Path $ScriptDirectory "cli") --root $InstallRoot --force
    if ($LASTEXITCODE -ne 0) {
        Fail "cargo install failed with exit code $LASTEXITCODE"
    }
    Test-Installation
}

if (-not $NoPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $pathEntries = @()
    if (-not [string]::IsNullOrWhiteSpace($userPath)) {
        $pathEntries = @($userPath -split ";" | Where-Object { $_ })
    }
    $alreadyPresent = $pathEntries | Where-Object {
        [StringComparer]::OrdinalIgnoreCase.Equals($_, $BinaryDirectory)
    }
    if ($null -eq $alreadyPresent) {
        if ($DryRun) {
            Write-Host "+ add $BinaryDirectory to the user PATH"
        } else {
            [Environment]::SetEnvironmentVariable(
                "Path",
                (($pathEntries + $BinaryDirectory) -join ";"),
                "User"
            )
            $env:Path = "$BinaryDirectory;$env:Path"
            Write-Host "Added $BinaryDirectory to the user PATH"
        }
    }
}

if ($DryRun) {
    Write-Host "Dry-run complete: no files, PATH entries, or toolchains were changed"
} else {
    Write-Host "Try: zelyra run $(Join-Path $ScriptDirectory 'examples\fibonacci.zyl')"
}
