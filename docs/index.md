# Documentation index

This directory contains both user-facing and developer-facing documentation for
`disk-nix`.

## Pick An Audience

| Audience | Start here |
| --- | --- |
| Users and operators | [User docs](user-docs.md) |
| Contributors and maintainers | [Developer docs](developer-docs.md) |

## User Fast Path

| Step | Document |
| --- | --- |
| Learn the workflow | [User guide](user-guide.md) |
| Check supported scope | [Storage scope](storage-scope.md) |
| Review current maturity | [Feature status](status.md) |
| Use the CLI | [CLI](cli.md) |
| Declare NixOS storage | [NixOS module](nixos-module.md) |
| Prepare risky work | [Operator runbooks](operator-runbooks.md) |

## Developer Fast Path

| Step | Document |
| --- | --- |
| Understand internals | [Architecture](architecture.md) |
| Review contracts | [Compatibility](compatibility.md) |
| Work on planner behavior | [Planning](planning.md) |
| Work on apply reports | [CLI planning and apply](cli-plan-apply.md) |
| Work on module behavior | [NixOS module reference](nixos-module-reference.md) |
| Work on tests | [Integration tests](integration-tests.md) |

## Reference map

| Document | Use it for |
| --- | --- |
| [README](../README.md) | Project overview, development commands, CLI examples, and safety model. |
| [User docs](user-docs.md) | User and operator documentation map. |
| [Developer docs](developer-docs.md) | Contributor and maintainer documentation map. |
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
