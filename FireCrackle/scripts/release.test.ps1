[CmdletBinding()]
param([string]$ArchivePath)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$project = Split-Path $PSScriptRoot -Parent
foreach ($name in @('release.ps1', 'prepare-runtime.ps1', 'release.test.ps1', 'release-archive.ps1')) {
    $parseErrors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $name), [ref]$null, [ref]$parseErrors)
    if ($parseErrors.Count) { throw "$name parse errors: $parseErrors" }
}
$previousRef = $env:GITHUB_REF
try {
    $env:GITHUB_REF = ''
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'release.ps1') -ValidateOnly
    if ($LASTEXITCODE -ne 0) { throw 'Valid release configuration rejected.' }
    $env:GITHUB_REF = 'refs/tags/v9999.0.0'
    $ErrorActionPreference = 'Continue' # Expected native stderr from the deliberately rejected tag.
    $message = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot 'release.ps1') -ValidateOnly 2>&1
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -eq 0 -or "$message" -notmatch 'Tag must be') { throw 'Mismatched release tag was not rejected.' }
} finally { $env:GITHUB_REF = $previousRef }
. (Join-Path $PSScriptRoot 'release-archive.ps1')
$fixture = Join-Path $project ".release-cache/zip-tests/$([guid]::NewGuid().ToString('N'))"
$fixtureStage = Join-Path $fixture 'Example'
New-Item -ItemType Directory -Path (Join-Path $fixtureStage 'tools') -Force | Out-Null
'test payload' | Set-Content -LiteralPath (Join-Path $fixtureStage 'tools/notice.txt') -Encoding UTF8
$fixtureZip = Join-Path $fixture 'example.zip'
Write-ReleaseZip -Stage $fixtureStage -Destination $fixtureZip
$check = [IO.Compression.ZipFile]::OpenRead($fixtureZip)
try {
    if (!$check.GetEntry('Example/tools/notice.txt') -or @($check.Entries | Where-Object { $_.FullName.Contains('\') }).Count) {
        throw 'ZIP generator failed to produce canonical forward-slash entries.'
    }
} finally { $check.Dispose() }
if ($ArchivePath) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead((Resolve-Path -LiteralPath $ArchivePath))
    try {
        $version = (Get-Content -LiteralPath (Join-Path $project 'package.json') -Raw | ConvertFrom-Json).version
        $prefix = "FireCrackle-$version-windows-x64/"
        foreach ($name in @('FireCrackle.exe','README.txt','LICENSE.txt','THIRD_PARTY_NOTICES.txt','release.json','SHA256SUMS.txt','tools/ffmpeg.exe','tools/ffprobe.exe','tools/LICENSE','tools/SOURCE.txt')) {
            if (!$zip.GetEntry("$prefix$name")) { throw "Missing packaged file: $name" }
        }
        foreach ($entry in $zip.Entries) {
            if (!$entry.FullName.StartsWith($prefix) -or $entry.FullName -match '\.\.|labels\.jsonl|project\.json|\.mkv$') { throw "Unexpected archive entry: $($entry.FullName)" }
            $relative = $entry.FullName.Substring($prefix.Length)
            if ($relative -and !$relative.StartsWith('tools/') -and $relative -notin @('FireCrackle.exe','README.txt','LICENSE.txt','THIRD_PARTY_NOTICES.txt','release.json','SHA256SUMS.txt')) { throw "Unexpected app file: $relative" }
        }
        $reader = New-Object IO.StreamReader($zip.GetEntry("${prefix}SHA256SUMS.txt").Open())
        try { $lines = $reader.ReadToEnd() -split '\r?\n' } finally { $reader.Dispose() }
        foreach ($line in $lines) {
            if (!$line.Trim()) { continue }
            if ($line -notmatch '^([a-f0-9]{64})  (.+)$') { throw "Invalid checksum entry: $line" }
            $expected = $Matches[1]; $name = $Matches[2]
            $entry = $zip.GetEntry("$prefix$name")
            if (!$entry) { throw "Checksummed entry missing: $name" }
            $stream = $entry.Open(); $sha = [Security.Cryptography.SHA256]::Create()
            try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }
            finally { $stream.Dispose(); $sha.Dispose() }
            if ($actual -ne $expected) { throw "Checksum mismatch: $name" }
        }
    } finally { $zip.Dispose() }
    Write-Host "Archive contents and checksums verified: $ArchivePath"
}
Write-Host 'Release pipeline checks passed.'
# The deliberately rejected tag leaves LASTEXITCODE=1. Report successful
# assertions explicitly so callers such as GitHub Actions do not reuse it.
exit 0
