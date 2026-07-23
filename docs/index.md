# Documentation index

This directory is the operator and developer manual for `disk-nix`. Read it in
the order below when evaluating the project as a storage lifecycle manager.

## Fast path

- Start with [Feature status](status.md) for the current implementation state,
  proof level, and known hardening areas.
- Use [User guide](user-guide.md) for task-oriented workflows from discovery
  through apply and recovery.
- Use [Storage scope](storage-scope.md) to see which Linux storage domains are
  modeled, inspected, planned, and mutated.
- Use [CLI](cli.md) when integrating with automation. JSON output is the stable
  interface; human text is presentation.
- Use [CLI planning and apply](cli-plan-apply.md) for planner, apply,
  validation, rollback, and report contracts.
- Use [NixOS module](nixos-module.md) when declaring storage from NixOS.
- Use [NixOS module reference](nixos-module-reference.md) for the full typed
  option example and domain-specific module behavior.
- Use the integration detail docs for harness catalogs.

[Integration smoke harnesses](integration-smoke-harnesses.md) covers host-backed
and lab-backed harnesses. [Integration failure recovery](integration-failure-recovery.md)
covers synthetic failure cases and recovery reports.
- Use [Operator runbooks](operator-runbooks.md) before any replacement,
  rollback, recovery, degraded-array, or shared-storage operation.

## Reference map

| Document | Use it for |
| --- | --- |
| [README](../README.md) | Project overview, development commands, CLI examples, and safety model. |
| [User guide](user-guide.md) | Operator workflows for inspecting, planning, applying, recovering, using NixOS, and testing safely. |
| [Architecture](architecture.md) | Data flow, crate boundaries, storage graph, probe adapters, and safety layers. |
| [Storage scope](storage-scope.md) | Domain-by-domain storage awareness and update operation coverage. |
| [CLI](cli.md) | Command index, discovery, focused views, inspection, and capabilities. |
| [CLI planning and apply](cli-plan-apply.md) | Plan/apply behavior, validation, rollback recipes, and report fields. |
| [Planning](planning.md) | Risk classification, dependency ordering, reconciliation, lifecycle grouping, and apply policy. |
| [NixOS module](nixos-module.md) | Declarative entrypoint, generated files, apply modes, and policy options. |
| [NixOS module reference](nixos-module-reference.md) | Full typed option example, generated-file details, and domain-specific module behavior. |
| [Integration tests](integration-tests.md) | Destructive opt-ins, suite entrypoints, and flake coverage. |
| [Integration failure recovery](integration-failure-recovery.md) | Synthetic failed-command catalog and recovery report expectations. |
| [Integration smoke harnesses](integration-smoke-harnesses.md) | Host-backed, VM-backed, and lab-backed harness details. |
| [Operator runbooks](operator-runbooks.md) | Human procedures for high-risk storage changes and failed apply recovery. |
| [Compatibility](compatibility.md) | Spec versioning, JSON output compatibility, CLI text policy, and safety invariants. |
| [Feature checklist](feature-checklist.md) | Requirement-by-requirement completion evidence. |

## Verification

The repository has two documentation verification paths:

- `nix build .#checks.x86_64-linux.documentation` validates required coverage
  markers across README and docs.
- `node scripts/render-docs.mjs` renders the markdown set to
  `build/docs-site/` for browser review.

After rendering, inspect at least the index, status, CLI, NixOS module,
integration tests, and feature checklist pages in a browser. The rendered site
uses a persistent document navigation rail, an on-page table of contents, wide
code blocks with horizontal scrolling, and responsive mobile layout so long
storage examples remain readable.
