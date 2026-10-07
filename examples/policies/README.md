# Policy Examples

Sample YAML governance policy files for AgentMesh.

Each file in this directory is a self-contained policy configuration that demonstrates how to express a particular class of security or compliance control using the policy engine. They are intended as starting points — review and adapt them for your environment before deploying to production.

## Using this directory

1. **Browse** the `.yaml` files to find a scenario close to what you need. Each file opens with a comment block describing what it covers and any caveats.
2. **Copy** the file into your own project (or reference it by path) and edit the rules, thresholds, and matchers to fit your requirements.
3. **Load** the policy into an agent workflow via the governance runtime. The [Quickstart](../quickstart/) shows runnable end-to-end examples that consume policies from this directory.

## Policy format

Agent Control Specification manifests follow the schema maintained in the [standalone Agent Control Specification project](https://github.com/responsibleai/agent-control-spec). Refer to its [manifest schema](https://github.com/responsibleai/agent-control-spec/blob/main/spec/schema/manifest.schema.json) for the supported fields, matchers, and enforcement actions.

## Related

- [Quickstart](../quickstart/) — runnable examples that load policies from this directory
- [Policy Engine tutorial](../../docs/tutorials/01-policy-engine.md) — walkthrough of how policies are evaluated
