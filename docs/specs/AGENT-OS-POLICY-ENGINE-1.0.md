---
title: Native Agent Policy Runtime
last_reviewed: 2026-07-11
owner: agt-maintainers
---

# Native Agent Policy Runtime

The former Agent OS policy contract is removed. Agent Governance Toolkit uses
the standalone [Agent Control Specification runtime](https://github.com/responsibleai/agent-control-spec).

Author a native manifest, bind policies to intervention points, and construct
the language SDK `AgentControl` or Python `AgentControl`. The normative contract
is the upstream [Agent Control Specification](https://github.com/responsibleai/agent-control-spec/blob/main/spec/SPECIFICATION.md).

Use `agt migrate v4-to-v5` once for an existing project. Runtime modules do not
load legacy governance files.
