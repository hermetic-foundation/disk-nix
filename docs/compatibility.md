# Compatibility policy

`disk-nix` is pre-1.0 software, but storage automation needs predictable
interfaces before it can be trusted. This document defines the compatibility
contract for specs, JSON reports, CLI text, NixOS options, and generated files.

## Versioned spec contract

Desired storage documents use contract version `1`.

The parser accepts either a direct spec:

```json
{
  "version": 1,
  "filesystems": {}
}
```

or the NixOS module wrapper:

```json
{
  "version": 1,
  "spec": {
    "version": 1,
    "filesystems": {}
  },
  "apply": {}
}
```

Omitting `version` is currently equivalent to `version = 1`. When both
top-level `version` and `spec.version` are present, they must match. Unsupported
future versions must be rejected before planning, applying, or generating shell
scripts.

Within version `1`, compatible additions are allowed:

- new optional fields
- new storage domains
- new lifecycle operations that are blocked or non-ready by default until
  renderers and safety gates are implemented
- new JSON report fields
- additional enum values in machine-readable report fields
- stricter diagnostics for ambiguous or unsafe input

Incompatible spec changes require a new version:

- removing or renaming an accepted field
- changing the meaning of an existing field
- changing default safety policy for an existing operation
- treating an omitted field as a different desired state
- allowing a previously blocked destructive or potential-data-loss operation by
  default

## Migration rules

When a future contract version is added, migration must be explicit and
reviewable. A migration path should include:

- parser tests for old and new versions
- a documented field mapping for changed names or semantics
- a command or documented workflow that renders the migrated spec without
  applying it
- examples covering the changed shape
- NixOS module assertions or warnings when an option maps to changed behavior

Automatic migration must not hide destructive, rollback, format, shrink, or
device-removal behavior. If a migration changes mutation semantics, the migrated
plan must require normal policy gates and should prefer a blocked or non-ready
report with advice over guessing.

## JSON output contracts

JSON output is the automation interface. This includes:

- `topology --json`
- focused graph views such as `devices --json`, `filesystems --json`, `lvm --json`, `zfs --json`, `vdo --json`, `iscsi --json`, `nfs --json`, and
  `usage --json`
- `inspect --json`
- `probe-status --json`
- `capabilities --json`
- `plan --json`
- `apply --json`
- `validate --json`
- generated report files from `--report-out`

Consumers should ignore unknown object fields. `disk-nix` may add fields to
JSON objects without changing the spec version. Existing field names should not
be reused for different meanings.

Enum-like strings are expected to grow as support expands. Consumers should
treat unknown enum values as unsupported or needing review, not as safe.

Arrays whose order carries operational meaning must document that meaning. For
example, command plans and execution results are ordered. Arrays used as
inventories, such as tool requirements, should be treated as sorted but not
semantically ordered.

## CLI text output

Human-readable tables, tree views, and status lines are diagnostic surfaces, not
stable machine interfaces. Scripts should use JSON output or generated report
files instead of parsing text output.

Text output may change to improve readability, add fields, or clarify safety
warnings without a spec version bump.

## NixOS options

NixOS module options are part of the declarative user interface. Compatible
changes include:

- adding optional declarations
- adding assertions for ambiguous identities
- deriving additional steady-state NixOS options from existing active
  declarations
- adding read-only validation or report artifacts

Incompatible option changes require migration notes and, where possible, module
warnings or assertions:

- renaming or removing options
- changing the default value of an apply safety option
- changing whether an operation is treated as active steady state or imperative
  lifecycle work
- changing generated file paths or service ordering in a way that affects boot
  or activation behavior

Lifecycle declarations that request teardown, rollback, shrink, format, or other
risky mutation must remain in the planner/apply path and should not be silently
translated into steady-state NixOS configuration.

## Generated artifacts

Generated artifacts include:

- shell scripts from `--script-out`
- JSON reports from `--report-out`
- the installed JSON schema
- completions
- the manpage
- `/etc/disk-nix/spec.json` from the NixOS module

Generated shell scripts are review artifacts. They may gain comments,
verification commands, or safety notes, but command ordering and mutating
commands must continue to reflect the execution report.

Generated JSON reports follow the JSON output policy. The installed schema must
match `disk-nix schema` for the same build.

## Safety invariants

Compatibility must not weaken the safety model. Across versions and output
changes:

- destructive work must be explicit
- potential data loss must be policy-gated
- unsupported actions must stay blocked or non-ready
- unresolved targets, desired sizes, identities, or command renderers must not be
  guessed
- current-topology reconciliation may suppress safe no-op work only when the
  graph proves it is already satisfied and no warning diagnostics are present
- execution must stop on the first failed command
- verification commands must be read-only
