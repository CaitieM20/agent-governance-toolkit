---
title: ACS scanner policy consolidation audit
last_reviewed: 2026-10-07
owner: agt-maintainers
---

# ACS scanner policy consolidation audit

## Which dependencies changed and why

No dependencies, dependency manifests, lockfiles, vendored content, or
implementation code change in this scanner-only PR. The only changes are
the supply-chain workflow, its Cargo scanner configuration, and this audit.

PR #4262 removes `policy-engine/` and changes dependencies. Its scanner
edits trigger the supply-chain gate that refuses co-modification of scanners
and dependency manifests. This preparatory PR must land first. After rebasing
#4262 onto it, the removal PR can retain this workflow and canonical scanner
config unchanged without creating a gap in source-policy coverage.

Before consolidation, parsing both `policy-engine/deny.toml` and
`agent-governance-rust/deny.toml` confirms every policy section is identical.
The canonical `agent-governance-rust/deny.toml` retains all settings unchanged.
The redundant `policy-engine/deny.toml` is deleted only after every workflow
source check is explicitly pointed at the canonical config.

## Security advisory relevance

No dependency versions change, so there is no new CVE or advisory exception.
The advisory ignore list remains empty. Source policy still denies unknown
registries and unknown Git sources, allows only the crates.io index, and
has an empty Git allowlist. License and ban settings are also unchanged.

CI continues to run `cargo-deny --all-features check sources` for all four
existing manifests using the same checksum-verified cargo-deny release.
Advisory, license, and ban checks are not added or removed from CI.

| Manifest | Coverage after preparation |
| --- | --- |
| `policy-engine/Cargo.toml` | Runs whenever this manifest exists after checkout |
| `agent-governance-rust/Cargo.toml` | Always runs |
| `policy-engine/examples/coding_agent/app/Cargo.toml` | Runs whenever this manifest exists after checkout |
| `benchmarks/prompt-injection/harness/agt-rules-baseline/Cargo.toml` | Always runs |

Only the two manifests scheduled for removal receive existence conditions.
While either exists, its source check runs and any failure still fails the
job. After removal, only the corresponding check is skipped. The retained
Rust SDK and benchmark checks have no existence condition and remain
mandatory. The scanner/dependency co-modification trip-wire stays unchanged.

## Breaking change risk assessment

No public API or runtime behavior changes. Consolidation cannot relax policy
because the parsed configurations are equivalent and the retained settings
are unchanged. Using explicit config paths avoids dependence on the deleted
config while `policy-engine/` still exists.

Validation covers workflow YAML and expressions, shell syntax, action pinning
and read-only permissions, parsed TOML policy equivalence, and simulated
execution of the source-check steps with both retiring manifests present,
each independently absent, and both absent. A failing source scanner must
remain a failing step. The trip-wire is exercised with scanner-only paths
and a negative control containing a dependency manifest. No manifest or
lockfile is changed to simulate removal.

The required landing order is this scanner-only PR, followed by a rebase of
#4262 that removes its scanner changes. This PR does not edit #4262.
