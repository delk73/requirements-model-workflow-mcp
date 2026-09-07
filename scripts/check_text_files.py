#!/usr/bin/env python3
"""Check line-ending invariants for tracked, policy-covered text files."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Iterable


POLICY_EXTENSIONS = {
    ".json",
    ".md",
    ".ps1",
    ".py",
    ".rs",
    ".sh",
    ".toml",
    ".txt",
    ".yaml",
    ".yml",
}
POLICY_FILENAMES = {".editorconfig", ".gitignore", "Cargo.lock", "LICENSE"}


def is_policy_path(path: Path) -> bool:
    return path.name in POLICY_FILENAMES or path.suffix in POLICY_EXTENSIONS


def check_bytes(content: bytes) -> list[str]:
    """Return invariant violations for one file's raw bytes."""
    if not content:
        return []

    violations = []
    if b"\r" in content:
        violations.append("contains carriage-return bytes")
    if content[-1:] != b"\n":
        violations.append("does not end with exactly one final LF")
    elif content.endswith(b"\n\n"):
        violations.append("has more than one trailing LF")
    return violations


def tracked_paths(repo_root: Path) -> Iterable[Path]:
    result = subprocess.run(
        ["git", "-C", str(repo_root), "ls-files", "-z"],
        check=True,
        stdout=subprocess.PIPE,
    )
    for raw_path in result.stdout.split(b"\0"):
        if raw_path:
            yield repo_root / Path(raw_path.decode("utf-8"))


def check_repository(repo_root: Path) -> list[tuple[Path, list[str]]]:
    violations = []
    for path in tracked_paths(repo_root):
        relative_path = path.relative_to(repo_root)
        if not is_policy_path(relative_path):
            continue
        file_violations = check_bytes(path.read_bytes())
        if file_violations:
            violations.append((relative_path, file_violations))
    return violations


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    violations = check_repository(repo_root)
    for path, file_violations in violations:
        for violation in file_violations:
            print(f"{path}: {violation}", file=sys.stderr)
    return 1 if violations else 0


if __name__ == "__main__":
    raise SystemExit(main())
