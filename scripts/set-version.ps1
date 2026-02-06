param(
  [Parameter(Mandatory = $true)]
  [string] $Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $repoRoot

function Assert-SemVer {
  param([Parameter(Mandatory = $true)][string]$Value)

  $pattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$'
  if (-not ($Value -match $pattern)) {
    throw "Invalid semver: $Value"
  }
}

function Read-TextFile {
  param([Parameter(Mandatory = $true)][string]$Path)
  if (-not (Test-Path -LiteralPath $Path)) {
    throw "File not found: $Path"
  }
  $content = [System.IO.File]::ReadAllText((Resolve-Path -LiteralPath $Path), [System.Text.Encoding]::UTF8)
  if ($content.Length -gt 0 -and $content[0] -eq [char]0xFEFF) {
    $content = $content.Substring(1)
  }
  return $content
}

function Write-TextFileUtf8NoBom {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$Content
  )

  $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
  [System.IO.File]::WriteAllText((Resolve-Path -LiteralPath $Path), $Content, $utf8NoBom)
}

function Set-JsonVersionInPlace {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$NewVersion
  )

  $text = Read-TextFile -Path $Path

  $re = [regex]'"version"\s*:\s*"([^"]+)"'
  $m = $re.Match($text)
  if (-not $m.Success) {
    throw "Could not find `"version`" field in JSON: $Path"
  }

  $updated = $re.Replace($text, ('"version": "{0}"' -f $NewVersion), 1)
  Write-TextFileUtf8NoBom -Path $Path -Content $updated
}

function Set-CargoPackageVersionInPlace {
  param(
    [Parameter(Mandatory = $true)][string]$Path,
    [Parameter(Mandatory = $true)][string]$NewVersion
  )

  $text = Read-TextFile -Path $Path
  $newline = "`n"
  if ($text -match "`r`n") { $newline = "`r`n" }

  $lines = $text -split "`r?`n"

  $inPackage = $false
  $changed = $false

  for ($i = 0; $i -lt $lines.Length; $i++) {
    $line = $lines[$i]

    if ($line -match '^\s*\[package\]\s*$') {
      $inPackage = $true
      continue
    }

    if ($inPackage -and $line -match '^\s*\[.+\]\s*$') {
      $inPackage = $false
    }

    if ($inPackage -and (-not $changed) -and $line -match '^\s*version\s*=\s*".*"\s*$') {
      $indent = ($line -replace '^(\s*).+$', '$1')
      $lines[$i] = ($indent + 'version = "' + $NewVersion + '"')
      $changed = $true
    }
  }

  if (-not $changed) {
    throw "Could not update [package].version in Cargo.toml: $Path"
  }

  $updated = ($lines -join $newline)
  Write-TextFileUtf8NoBom -Path $Path -Content $updated
}

Assert-SemVer -Value $Version

Set-JsonVersionInPlace -Path "package.json" -NewVersion $Version
Set-JsonVersionInPlace -Path "src-tauri/tauri.conf.json" -NewVersion $Version
Set-CargoPackageVersionInPlace -Path "src-tauri/Cargo.toml" -NewVersion $Version

Write-Host ("Version synced to {0}" -f $Version)
