---
title: ACS scanner and CI consolidation audit
last_reviewed: 2026-10-07
owner: agt-maintainers
---

# ACS scanner and CI consolidation audit

AGT plans to remove the in-repository `policy-engine/` implementation and
consume compatible packages from the standalone ACS project instead. This
change prepares for that removal by consolidating the identical Cargo
source-policy configurations into `agent-governance-rust/deny.toml` and
removing policy-engine build, test, packaging, and publishing jobs from CI
and release automation. It does not remove the implementation or retarget
AGT's consumer dependencies.

## Which dependencies changed and why

No dependencies, dependency manifests, lockfiles, vendored content, or runtime
implementation change. Both `deny.toml` files had identical policy settings;
the retained configuration preserves those settings, and the duplicate
`policy-engine/deny.toml` is removed. Every Cargo source-policy check now
explicitly selects the retained configuration, so removing `policy-engine/`
later will not remove the policy used by AGT's Rust workspace or benchmark.

The generated `policy-engine-ci` workflow and its source definition are
removed. GitHub Actions and ESRP release workflows no longer build, package,
or publish ACS artifacts from the in-tree Rust, Python, Node, or .NET SDK
directories; the release manifest no longer lists those artifacts.

The `cargo-source-policy` check continues to inspect the two policy-engine
manifest paths while they exist and skips only those exact steps after their
removal. The agent-governance Rust workspace and prompt-injection benchmark
checks remain unconditional. Every remaining check explicitly selects the
shared source-policy file. The scanner/dependency co-modification trip-wire
is unchanged. Validation covers scanner-only, dependency-only, and combined
path cases without adding a new test file.

Standalone-consumer CI installation is intentionally not added here. The
AGT Python consumer currently requires `agent-control-specification>=0.4.0b0`,
while the published PyPI package is only `0.3.1b1`; installing that older
release would not satisfy the requirement or establish API compatibility. The
standalone Rust crate is a separate package and API. PR2 must retarget the
consumer manifests and consumer CI together once a compatible standalone
release is available. Until then, AGT's existing consumer tests retain their
current source-based setup; this PR does not guess a package version or
fabricate compatibility.

## Security advisory relevance

No dependency versions changed, so this change introduces no new CVE or
advisory exception. The `[sources]` policy remains unchanged: unknown registries
and Git sources are denied, only the crates.io index is allowed, and the Git
allow-list remains empty. Advisory, license, and ban settings are unchanged;
CI continues to run only the deterministic `check sources` gate.

## Breaking change risk assessment

No public API or runtime behavior changes. The risk is limited to CI coverage
and release sequencing: in-tree ACS SDK build/test/package jobs are removed
before the separate PR that retargets AGT consumers. Do not replace their
consumer setup with the currently incompatible PyPI release. The downstream
consumer CI migration must land with the manifest retarget in PR2 and use only
a published, API-compatible package. During the coordinated rollout, the
in-tree source-policy checks remain active until each retired manifest is
removed, and the retained Rust/benchmark source-policy checks remain required.
