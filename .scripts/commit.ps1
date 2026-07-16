# Plumbing commit — this repo's `git commit` porcelain hangs on the working-tree scan.
# Usage: .scripts/commit.ps1 -Message "type: subject"
param([Parameter(Mandatory=$true)][string]$Message)
$ErrorActionPreference = "Stop"
git add -A
$tree = (git write-tree).Trim()
$parent = (git rev-parse HEAD).Trim()
$commit = ($Message | git commit-tree $tree -p $parent).Trim()
git update-ref HEAD $commit
Write-Output "committed $commit"
git log --oneline -1
