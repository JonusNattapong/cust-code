# Errors

Failures hit while building, and what actually fixed them. An entry earns its place if the
cause was not obvious from the error message.

Format: symptom → cause → fix. Newest first.

---

## 2026-08-08 — `mv: cannot move ... Permission denied` on a clean repo

**Symptom.** Renaming the project directory failed from Git Bash `mv`, then from PowerShell
`Rename-Item`, even after `rm -rf target/`. No file was open in an editor.

**Cause.** `rust-analyzer.exe` (two instances) held handles on the directory. Windows blocks
directory rename while any process holds a handle inside it — unlike POSIX, where an open
file does not prevent renaming its parent.

**Fix.** `Copy-Item -Recurse` to the new name, then `Remove-Item -Recurse -Force` on the old
one, which succeeded because nothing new was opening under it.

## 2026-08-08 — `bash: fork: Permission denied` mid-session

**Symptom.** Git Bash commands intermittently failed with
`CreateProcessW failed ... errno 13` / `fork: Permission denied`, then worked again later.

**Cause.** Not diagnosed. Git Bash `fork()` emulation on Windows fails under conditions we
did not isolate — possibly antivirus or handle pressure.

**Fix / workaround.** Re-run in PowerShell, which was unaffected every time. Not worth
chasing further unless it becomes frequent; note it here so the next occurrence is
recognized rather than re-investigated.
