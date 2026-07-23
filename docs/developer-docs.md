# Developer docs

Use these documents when changing `disk-nix`, reviewing implementation scope, or
maintaining its tests and compatibility contract.

## Project Shape

| Document | Use it for |
| --- | --- |
| [Architecture](architecture.md) | Data flow, crate boundaries, graph model, probes, and safety layers. |
| [Feature checklist](feature-checklist.md) | Requirement-by-requirement completion evidence. |
| [Feature status](status.md) | Implementation status and remaining hardening areas. |
| [Storage scope](storage-scope.md) | Storage-domain model and discovery coverage. |

## Contracts

| Document | Use it for |
| --- | --- |
| [Compatibility](compatibility.md) | Versioning, JSON contracts, CLI text policy, generated artifacts. |
| [Planning](planning.md) | Planner semantics, risk classes, dependency ordering, reconciliation. |
| [CLI planning and apply](cli-plan-apply.md) | Report fields, command rendering, rollback recipes, replay gates. |
| [NixOS module reference](nixos-module-reference.md) | Typed option behavior and native NixOS derivation rules. |

## Test And Proof Maintenance

| Document | Use it for |
| --- | --- |
| [Integration tests](integration-tests.md) | Suite entrypoints, destructive guards, flake coverage checks. |
| [Integration smoke harnesses](integration-smoke-harnesses.md) | Harness matrix and lab requirements. |
| [Integration failure recovery](integration-failure-recovery.md) | Synthetic failure domains and report contract. |

## User-Facing Behavior

| Document | Use it for |
| --- | --- |
| [User guide](user-guide.md) | Expected operator workflows. |
| [Operator runbooks](operator-runbooks.md) | Required guidance for high-risk operations. |
| [CLI](cli.md) | Stable command surface and focused read-only views. |
