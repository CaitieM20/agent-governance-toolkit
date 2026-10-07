---
title: "Dependency audit: retire the in-repository ACS implementation"
last_reviewed: 2026-10-07
owner: agt-maintainers
---

# Retire the in-repository ACS implementation

## Which dependencies changed and why

This is a repository-boundary migration, not a routine version refresh. The
Agent Control Specification implementation and its SDKs have graduated to
[`responsibleai/agent-control-spec`](https://github.com/responsibleai/agent-control-spec).
AGT removes the in-tree `policy-engine` workspace and its language bindings,
examples, and packaging manifests; AGT integrations are intended to consume
published ACS artifacts instead.

The lockfile footprint is unusually large:

| Lockfile | Before | After | Change |
| --- | ---: | ---: | ---: |
| `agent-governance-rust/Cargo.lock` package records | 341 | 312 | 4 added, 33 removed |
| `policy-engine/Cargo.lock` package records | 446 | removed | 446 removed |
| `policy-engine/examples/coding_agent/app/Cargo.lock` package records | 266 | removed | 266 removed |
| `policy-engine/sdk/node/package-lock.json` package records | 144 | removed | 144 removed |

Across these lockfiles, the recorded graph shrinks by 885 package records,
including four additions in the surviving Rust workspace. The Rust consumer
lock removes the standalone `agent-control-spec` and `agent-hooks-sdk` entries
that were resolved through the in-tree compatibility source, along with
manifest-validation and HTTP/TLS transitive packages. It adds the registry
packages `agent_control_specification` and
`agent_control_specification_core` at `0.3.1-beta.0`, plus `ureq` 2.12.1 and
`webpki-roots` 0.26.11.

The removed subtree also contained 11 Cargo manifests, 2 Python project
manifests, 12 Node package manifests, and 12 .NET project files. The
ACS-specific example requirement files are removed with the implementations
that consumed them. This entry records the dependency and lockfile changes for
the retirement; source-policy workflow consolidation is tracked in a separate
scanner/CI change.

## Security advisory relevance

No CVE or RustSec advisory is addressed by this change. The in-tree ACS code
and its private dependency graphs are removed from AGT. The surviving Rust
workspace records the exact published `agent_control_specification`
registry-crate version shown above; that version is not a claim that the
standalone project's newer `agent-control-spec` Rust API is compatible. The
standalone package retarget and its API compatibility must be reviewed
separately before release.

No third-party source is added by the lockfile changes; registry lock entries
retain their generated checksums. This audit does not make an attribution
determination about the YAML normalization code retained by the Rust
integration.

## Breaking change risk assessment

High. The removed `policy-engine` modules and SDK packages are no longer
provided by AGT; consumers must migrate to the standalone ACS project and its
published packages. The Rust integration also changes its ACS dependency and
observable API surface. The current Rust lock resolves the older
`agent_control_specification` 0.3.1 beta line, so it must not be represented as
equivalent to the standalone 0.4 alpha contract. The package-level migration
guidance and release compatibility need to be completed and validated before
these changes are considered release-ready.
