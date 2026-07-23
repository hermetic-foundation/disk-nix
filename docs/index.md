# Documentation index

This directory separates operator documentation from contributor
documentation.

## Pick An Audience

| Audience | Start here |
| --- | --- |
| Users and operators | [User docs](user/index.md) |
| Contributors and maintainers | [Developer docs](developer/index.md) |

## User Fast Path

| Step | Document |
| --- | --- |
| Learn the workflow | [User guide](user/user-guide.md) |
| Check supported scope | [Storage scope](user/storage-scope.md) |
| Review current maturity | [Feature status](user/status.md) |
| Use the CLI | [CLI](user/cli.md) |
| Declare NixOS storage | [NixOS module](user/nixos-module.md) |
| Prepare risky work | [Operator runbooks](user/operator-runbooks.md) |

## Developer Fast Path

| Step | Document |
| --- | --- |
| Understand internals | [Architecture](developer/architecture.md) |
| Review contracts | [Compatibility](developer/compatibility.md) |
| Work on planner behavior | [Planning](developer/planning.md) |
| Work on apply reports | [CLI planning and apply](user/cli-plan-apply.md) |
| Work on module behavior | [NixOS module reference](user/nixos-module-reference.md) |
| Work on tests | [Integration tests](developer/integration-tests.md) |

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
