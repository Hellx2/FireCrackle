# Releasing FireCrackle

Run from the repository root with Git, Python 3 and an authenticated GitHub CLI:

```powershell
python FireCrackle/scripts/prepare_release.py --version 0.1.1 --source main
```

Choose the committed branch or SHA containing the intended release. Uncommitted
development edits are excluded. The command creates `codex/release-v0.1.1` in
`.release-worktrees/v0.1.1`, updates the app and both lockfile versions, commits
only those version files, pushes that branch, and starts the release workflow.
It never switches or stages the development checkout. Agents can continue the
next version there. Do not edit a release workspace once its builds start.

The workflow builds and tests Windows x64 ZIP/setup and Linux x86_64
AppImage/Debian packages from the same frozen commit. Linux packages get content,
dependency and virtual-display startup checks. The publisher checks build
provenance and local/server SHA-256 hashes before publishing through a draft.
Both platforms must pass. Repository visibility and access remain unchanged.

For build-only testing, dispatch **FireCrackle release** manually with **Publish**
off (the default); choose one or both platforms. A `v*` tag also builds and
publishes both, so do not tag unfinished work. Tags must match the source version.

If a build fails, fix the release branch and run the workflow again. If upload
fails, inspect the draft and rerun the failed publish job; publication retries
verify the same commit and bytes. Once published, assets are immutable: fixes
need a new version. A draft targeting a different commit is deliberately rejected;
inspect/remove that draft before rebuilding a changed cut. Never force-move a
published tag. Existing release workspace/branch names are refused on a second
cut, preserving evidence instead of overwriting it.

Merge the release infrastructure back into development separately from the frozen
release tag. Keep the isolated workspace until publication is verified, then use
`git worktree remove` on that exact workspace after checking it is clean.
