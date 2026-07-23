# User docs

Use these documents when operating `disk-nix` or declaring storage on a NixOS
host.

## Start Here

| Document | Use it for |
| --- | --- |
| [User guide](user-guide.md) | Operator workflow from discovery through apply and recovery. |
| [Feature status](status.md) | Current implementation state, proof level, and hardening notes. |
| [Storage scope](storage-scope.md) | Supported storage domains and lifecycle operation coverage. |
| [Operator runbooks](operator-runbooks.md) | Human procedures for high-risk storage changes. |

## Interfaces

| Document | Use it for |
| --- | --- |
| [CLI](cli.md) | Discovery commands, focused views, inspection, and capabilities. |
| [CLI planning and apply](cli-plan-apply.md) | Plan/apply reports, validation, rollback recipes, and command readiness. |
| [NixOS module](nixos-module.md) | Declarative entrypoint, apply modes, policy, and generated files. |
| [NixOS module reference](nixos-module-reference.md) | Full typed option reference and domain behavior. |

## Safety References

| Document | Use it for |
| --- | --- |
| [Planning](../developer/planning.md) | Risk classification, lifecycle operations, reconciliation, and apply policy. |
| [Compatibility](../developer/compatibility.md) | Spec versions, JSON compatibility, CLI text policy, and safety invariants. |

## Testing Before Real Changes

| Document | Use it for |
| --- | --- |
| [Integration tests](../developer/integration-tests.md) | Destructive opt-ins, VM suite entrypoints, and disko example checks. |
| [Integration smoke harnesses](../developer/integration-smoke-harnesses.md) | Host-backed, VM-backed, and lab-backed harness behavior. |
| [Integration failure recovery](../developer/integration-failure-recovery.md) | Synthetic failure reports and rollback-review behavior. |
