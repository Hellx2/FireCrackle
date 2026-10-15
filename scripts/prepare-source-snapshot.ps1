[CmdletBinding()]
param([string]$Name = 'codex/clean-source-snapshot')
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = Split-Path $PSScriptRoot -Parent
Push-Location $root
try {
    function Invoke-Git([string[]]$Arguments) {
        $result = & git -c "safe.directory=$($root.Replace('\','/'))" @Arguments
        if ($LASTEXITCODE -ne 0) { throw "git failed: $($Arguments -join ' ')" }
        return $result
    }
    if (!$Name.StartsWith('codex/')) { throw 'Snapshot branch names must start with codex/.' }
    [void](Invoke-Git @('check-ref-format','--branch',$Name))
    if (@(Invoke-Git @('status','--porcelain')).Count) { throw 'Commit or reconcile all source changes before preparing a snapshot.' }
    & node (Join-Path $PSScriptRoot 'check-repository.mjs')
    if ($LASTEXITCODE -ne 0) { throw 'Repository boundary checks failed; no snapshot was created.' }
    & git -c "safe.directory=$($root.Replace('\','/'))" show-ref --verify --quiet "refs/heads/$Name"
    if ($LASTEXITCODE -eq 0) { throw "Branch $Name already exists; choose a new name. Existing branches are never overwritten." }
    $tree = (Invoke-Git @('rev-parse','HEAD^{tree}')).Trim()
    $source = (Invoke-Git @('rev-parse','HEAD')).Trim()
    $commit = (Invoke-Git @('commit-tree',$tree,'-m','Prepare clean source snapshot','-m',"Source tree from $source. Private development history and generated datasets are intentionally not included.")).Trim()
    [void](Invoke-Git @('update-ref',"refs/heads/$Name",$commit,('0' * 40)))
    if ((Invoke-Git @('rev-list','--count',$Name)).Trim() -ne '1') { throw 'Snapshot unexpectedly contains parent history.' }
    if ((Invoke-Git @('rev-parse',"$Name^{tree}")).Trim() -ne $tree) { throw 'Snapshot source tree differs from the verified tree.' }
    Write-Host "Prepared $Name at $commit (one commit, source tree unchanged)."
    Write-Host 'Current checkout and all private history are preserved. Nothing was pushed or made public.'
} finally { Pop-Location }
