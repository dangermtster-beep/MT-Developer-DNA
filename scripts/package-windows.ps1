[CmdletBinding()]
param(
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Version = '0.8.1'
$PackageName = "mt-developer-dna-v$Version-windows-x86_64"
$StagingRoot = Join-Path $ProjectRoot 'release-staging'
$PackageRoot = Join-Path $StagingRoot $PackageName
$DistRoot = Join-Path $ProjectRoot 'dist'
$Archive = Join-Path $DistRoot "$PackageName.zip"
$ChecksumFile = Join-Path $DistRoot "$PackageName.sha256.txt"

if (-not $SkipBuild) {
    & cargo build --workspace --release
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
}

$Binary = Join-Path $ProjectRoot 'target\release\dna-cli.exe'
if (-not (Test-Path -LiteralPath $Binary -PathType Leaf)) {
    throw "Release binary not found: $Binary"
}

$ResolvedStagingRoot = [System.IO.Path]::GetFullPath($StagingRoot)
$ResolvedPackageRoot = [System.IO.Path]::GetFullPath($PackageRoot)
if (-not $ResolvedPackageRoot.StartsWith($ResolvedStagingRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'Unsafe staging path.'
}

if (Test-Path -LiteralPath $PackageRoot) {
    Remove-Item -LiteralPath $PackageRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $PackageRoot -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $PackageRoot 'profiles') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $PackageRoot 'mt-developer-dna') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $PackageRoot 'docs') -Force | Out-Null
New-Item -ItemType Directory -Path $DistRoot -Force | Out-Null

Copy-Item -LiteralPath $Binary -Destination (Join-Path $PackageRoot 'dna-cli.exe')
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'profiles\universal.json') -Destination (Join-Path $PackageRoot 'profiles\universal.json')
# SKILL.md provider'a göre `dna-cli deploy` ile üretilir; pakette yalnız referans kaynağı bulunur.
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'mt-developer-dna\references') -Destination (Join-Path $PackageRoot 'mt-developer-dna\references') -Recurse
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'docs\INSTALL.md') -Destination (Join-Path $PackageRoot 'docs\INSTALL.md')
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'docs\PROVIDERS.md') -Destination (Join-Path $PackageRoot 'docs\PROVIDERS.md')
Copy-Item -LiteralPath (Join-Path $ProjectRoot "docs\RELEASE_NOTES_v$Version.md") -Destination (Join-Path $PackageRoot 'RELEASE_NOTES.md')
Copy-Item -LiteralPath (Join-Path $ProjectRoot "reports\v$Version-validation.md") -Destination (Join-Path $PackageRoot 'PUBLIC_RELEASE_VALIDATION.md')
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'README_TR.md') -Destination (Join-Path $PackageRoot 'README_TR.md')
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'LICENSE') -Destination (Join-Path $PackageRoot 'LICENSE')

if (Test-Path -LiteralPath $Archive) {
    Remove-Item -LiteralPath $Archive -Force
}
Compress-Archive -Path (Join-Path $PackageRoot '*') -DestinationPath $Archive -CompressionLevel Optimal

$Hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $Archive).Hash.ToLowerInvariant()
Set-Content -LiteralPath $ChecksumFile -Value "$Hash  $PackageName.zip" -Encoding ascii
Write-Output "PACKAGE=$Archive"
Write-Output "SHA256=$Hash"
Write-Output "CHECKSUM=$ChecksumFile"
