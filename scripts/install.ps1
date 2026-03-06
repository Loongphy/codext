$ErrorActionPreference = "Stop"

$Repo = "Loongphy/codext"
$InstallDir = "$env:USERPROFILE\bin"
$Asset = "codex-x86_64-pc-windows-msvc.zip"
$Url = "https://github.com/$Repo/releases/latest/download/$Asset"

$TmpBase = Join-Path $env:TEMP ("codex-install-" + [guid]::NewGuid().ToString("N"))
$ZipPath = Join-Path $TmpBase $Asset
$ExtractDir = Join-Path $TmpBase "out"

New-Item -ItemType Directory -Path $TmpBase -Force | Out-Null
New-Item -ItemType Directory -Path $ExtractDir -Force | Out-Null

try {
  Write-Host "Installing latest Codex from $Repo"
  Write-Host "Downloading asset: $Asset"

  Invoke-WebRequest -Uri $Url -OutFile $ZipPath
  Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force
  $ArchiveExe = Get-ChildItem -Path $ExtractDir -File -Recurse `
    | Where-Object { $_.Name -match "^codex.*\.exe$" } `
    | Select-Object -First 1

  if (-not $ArchiveExe) {
    throw "Failed to locate codex*.exe in archive $Asset"
  }

  New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
  Copy-Item $ArchiveExe.FullName (Join-Path $InstallDir "codex.exe") -Force

  $ExePath = Join-Path $InstallDir "codex.exe"
  Write-Host "Installed: $ExePath"
  & $ExePath --version

  if (-not (($env:Path -split ";") -contains $InstallDir)) {
    Write-Host "Add to PATH if needed:"
    Write-Host "  setx PATH `"$InstallDir;$env:Path`""
  }
}
finally {
  Remove-Item -Path $TmpBase -Recurse -Force -ErrorAction SilentlyContinue
}
