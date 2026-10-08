# Copyright (c) Microsoft Corporation.
# Licensed under the MIT License.
"""Regression tests for the ACS cargo source-policy workflow."""

from __future__ import annotations

import re
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
WORKFLOW = REPO_ROOT / ".github" / "workflows" / "supply-chain-check.yml"


def _workflow_tripwire_patterns() -> tuple[re.Pattern[str], re.Pattern[str]]:
    text = WORKFLOW.read_text(encoding="utf-8")
    scanner_line = next(
        line for line in text.splitlines() if "| grep -E '^(" in line
    )
    dependency_line = next(
        line for line in text.splitlines() if "| grep -E '(^|/)" in line
    )
    return (
        re.compile(scanner_line.split("| grep -E '", 1)[1].rsplit("'", 1)[0]),
        re.compile(dependency_line.split("| grep -E '", 1)[1].rsplit("'", 1)[0]),
    )


def _tripwire_rejects(changed_paths: list[str]) -> bool:
    scanner_pattern, dependency_pattern = _workflow_tripwire_patterns()
    scanner_hits = any(scanner_pattern.search(path) for path in changed_paths)
    dependency_hits = any(dependency_pattern.search(path) for path in changed_paths)
    return scanner_hits and dependency_hits


def test_scanner_only_change_passes_tripwire() -> None:
    assert not _tripwire_rejects(
        [
            ".github/workflows/supply-chain-check.yml",
            "agent-governance-rust/deny.toml",
            "policy-engine/deny.toml",
            "tests/ci/test_acs_scanner_workflow.py",
        ]
    )


def test_scanner_and_dependency_manifest_change_is_rejected() -> None:
    assert _tripwire_rejects(
        [
            "agent-governance-rust/deny.toml",
            "agent-governance-python/agt-policies/pyproject.toml",
        ]
    )


def test_dependency_only_change_does_not_trip_scanner_guard() -> None:
    assert not _tripwire_rejects(["agent-governance-rust/Cargo.toml"])


def test_every_cargo_manifest_uses_shared_policy_and_removable_paths_are_guarded() -> None:
    text = WORKFLOW.read_text(encoding="utf-8")
    assert text.count("--config agent-governance-rust/deny.toml") == 4
    assert "cargo-deny --manifest-path agent-governance-rust/Cargo.toml --config agent-governance-rust/deny.toml --all-features check sources" in text
    assert "hashFiles('policy-engine/Cargo.toml') != ''" in text
    assert (
        "hashFiles('policy-engine/examples/coding_agent/app/Cargo.toml') != ''"
        in text
    )
    assert "--all-features check sources" in text
