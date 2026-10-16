"""Cut a release in a separate worktree without changing the development checkout."""
import argparse
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
VERSION = r"\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?"
FILES = ["package.json", "package-lock.json", "src-tauri/tauri.conf.json",
         "src-tauri/Cargo.toml", "src-tauri/Cargo.lock"]


def run(*args, cwd=ROOT):
    return subprocess.run(args, cwd=cwd, check=True, text=True, capture_output=True).stdout.strip()


def bump_version(root, version):
    if not re.fullmatch(VERSION, version):
        raise ValueError("Use a semantic version such as 0.1.1")
    app = root / "FireCrackle"
    changes = {}
    for name in FILES[:3]:
        path = app / name
        data = json.loads(path.read_text(encoding="utf-8"))
        data["version"] = version
        if name == "package-lock.json":
            data["packages"][""]["version"] = version
        changes[path] = json.dumps(data, indent=2) + "\n"
    path = app / "src-tauri/Cargo.toml"
    changes[path], count = re.subn(r'(?m)^version\s*=\s*"[^"]+"',
                                   f'version = "{version}"', path.read_text(), count=1)
    if count != 1:
        raise ValueError("Missing desktop package version")
    path = app / "src-tauri/Cargo.lock"
    changes[path], count = re.subn(r'(\[\[package\]\]\s+name = "firecrackle"\s+version = ")[^"]+(")',
                                   lambda match: match[1] + version + match[2], path.read_text())
    if count != 1:
        raise ValueError("Missing or duplicate firecrackle lockfile package")
    # Validate every input before writing any file.
    for path, content in changes.items():
        path.write_text(content, encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--source", default="main", help="Committed branch/ref to freeze; dirty changes are excluded")
    parser.add_argument("--repo", default=None)
    args = parser.parse_args()
    if not re.fullmatch(VERSION, args.version) or args.source.startswith("-"):
        parser.error("Invalid version or source ref")
    repo = args.repo or run("gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner")
    if not re.fullmatch(r"[\w.-]+/[\w.-]+", repo):
        parser.error("Invalid repository")
    existing = subprocess.run(["git", "ls-remote", "--exit-code", "--tags", "origin", f"refs/tags/v{args.version}"],
                              cwd=ROOT, capture_output=True, text=True)
    if existing.returncode != 2:
        raise RuntimeError("Release tag already exists or remote could not be checked; nothing changed")
    commit = run("git", "rev-parse", "--verify", args.source + "^{commit}")
    branch = f"codex/release-v{args.version}"
    target = ROOT / ".release-worktrees" / f"v{args.version}"
    if target.exists():
        raise RuntimeError(f"Release workspace already exists: {target}. Inspect it before retrying.")
    run("git", "worktree", "add", "-b", branch, str(target), commit)
    bump_version(target, args.version)
    run("git", "add", "--", *["FireCrackle/" + name for name in FILES], cwd=target)
    dirty = run("git", "diff", "--cached", "--name-only", cwd=target)
    if dirty:
        run("git", "commit", "-m", f"Prepare FireCrackle v{args.version}", cwd=target)
    run("git", "push", "-u", "origin", branch, cwd=target)
    run("gh", "workflow", "run", "firecrackle-release.yml", "--repo", repo, "--ref", branch,
        "-f", "platform=all", "-f", "publish=true", cwd=target)
    print(f"Frozen release workspace: {target}\nBranch: {branch}\n"
          f"Builds and publication: https://github.com/{repo}/actions/workflows/firecrackle-release.yml\n"
          "Your development checkout was not switched or staged. Keep this workspace until the release completes.")


if __name__ == "__main__":
    main()
