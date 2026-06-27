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
- Use [NixOS module](nixos-module.md) when declaring storage from NixOS.
- Use [Operator runbooks](operator-runbooks.md) before any replacement,
  rollback, recovery, degraded-array, or shared-storage operation.

## Reference map

| Document | Use it for |
| --- | --- |
| [README](../README.md) | Project overview, development commands, CLI examples, and safety model. |
| [User guide](user-guide.md) | Operator workflows for inspecting, planning, applying, recovering, using NixOS, and testing safely. |
| [Architecture](architecture.md) | Data flow, crate boundaries, storage graph, probe adapters, and safety layers. |
| [Storage scope](storage-scope.md) | Domain-by-domain storage awareness and update operation coverage. |
| [CLI](cli.md) | Command contracts, JSON shapes, plan/apply behavior, validation, and rollback report fields. |
| [Planning](planning.md) | Risk classification, dependency ordering, reconciliation, lifecycle grouping, and apply policy. |
| [NixOS module](nixos-module.md) | Declarative options, generated files, steady-state inventory, apply modes, and policy options. |
| [Integration tests](integration-tests.md) | Host-backed and VM-backed smoke harnesses, destructive opt-ins, and flake coverage. |
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
