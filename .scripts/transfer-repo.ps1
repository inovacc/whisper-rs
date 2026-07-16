# Transfer dyammarcano/whisper-rs -> inovacc org, then repoint local origin.
$ErrorActionPreference = "Stop"

# HARD-RULE guard: never perform repo-lifecycle ops as the restricted account.
$who = (gh api user --jq .login).Trim()
if ($who -eq "dyammarcanoconsultor-ship-it") {
    throw "REFUSED: active gh account is the restricted 'dyammarcanoconsultor-ship-it'. Aborting repo transfer."
}
Write-Output "active account: $who (allowed)"

Write-Output "== transferring dyammarcano/whisper-rs -> inovacc =="
gh api -X POST repos/dyammarcano/whisper-rs/transfer -f new_owner=inovacc --jq '.full_name + \" (transfer accepted)\"'

Start-Sleep -Seconds 4
Write-Output "== verifying new location =="
gh api repos/inovacc/whisper-rs --jq '.full_name + \" | private=\" + (.private|tostring) + \" | default=\" + .default_branch'

Write-Output "== repointing local origin =="
git -C D:\new_page\whisper-rs remote set-url origin https://github.com/inovacc/whisper-rs.git
git -C D:\new_page\whisper-rs remote -v
