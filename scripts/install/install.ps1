param(
    [Parameter(Position=0)]
    [string]$Version = "latest"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

function Write-Step {
    param(
        [string]$Message
    )

    Write-Host "==> $Message"
}

function Normalize-Version {
    param(
        [string]$RawVersion
    )

    if ([string]::IsNullOrWhiteSpace($RawVersion) -or $RawVersion -eq "latest") {
        return "latest"
    }

    if ($RawVersion.StartsWith("rust-v")) {
        return $RawVersion.Substring(6)
    }

    if ($RawVersion.StartsWith("v")) {
        return $RawVersion.Substring(1)
    }

    return $RawVersion
}

if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
    Write-Error "npm is required to install Codext."
    exit 1
}

$resolvedVersion = Normalize-Version -RawVersion $Version
$packageSpec = "@loongphy/codex"
if ($resolvedVersion -ne "latest") {
    $packageSpec = "$packageSpec@$resolvedVersion"
}

Write-Step "Installing Codext from npm"
& npm install -g $packageSpec

Write-Step "Run: codext"
