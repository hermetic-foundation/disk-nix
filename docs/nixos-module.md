# NixOS module

The NixOS module is the primary declarative interface.

## Goals

- define storage once
- emit a normalized JSON planner spec
- derive regular NixOS options such as `fileSystems`, `swapDevices`, initrd
  LUKS devices, `zramSwap`, ZFS support, Btrfs support, iSCSI/NFS
  dependencies, and systemd units
- keep imperative mutation behind explicit policy

## Initial usage

```nix
{
  imports = [ inputs.disk-nix.nixosModules.default ];

  services.disk-nix = {
    enable = true;
    apply = {
      mode = "manual";
      allowGrow = true;
      allowPotentialDataLoss = false;
      allowDestructive = false;
      probeCurrent = true;
    };
    filesystems.root = {
      device = "/dev/disk/by-label/nixos-root";
      fsType = "xfs";
      mountpoint = "/";
      neededForBoot = true;
      resizePolicy = "grow-only";
      desiredSize = "100%";
    };
    zram = {
      enable = true;
      operation = "rescan";
      memoryPercent = 50;
      algorithm = "zstd";
    };
  };
}
```

Use [NixOS module reference](nixos-module-reference.md) for the full typed-option example, generated-file details, duplicate-identity rules, domain-specific service derivation, and lifecycle declaration behavior.

The module writes `/etc/disk-nix/spec.json` with top-level contract `version = 1`, installs the CLI and default storage tooling, and derives native NixOS storage options where declarations describe active steady state.

Imperative teardown, destructive, and under-specified lifecycle declarations stay in the disk-nix planner spec for reviewed apply instead of being re-added to native NixOS state.

Generated review artifacts include `/etc/disk-nix/steady-state.json`, `/etc/disk-nix/declarative-handoff.nix`, and `/etc/disk-nix/declarative-handoff-import.patch`.

## Apply modes

- `manual`: only install the spec and CLI
- `activation`: run apply-policy validation during activation

Destructive and potential-data-loss actions are refused by default. Set
`probeCurrent = true` to include current topology comparison in that validation
report.

Set `scriptOut`, `reportOut`, and `receiptOut` to emit reviewable shell,
JSON report, and invocation receipt artifacts. Set `failOnBlocked = false`
to report blocked policy without failing the unit.

Set `execute = true` to run ready, policy-allowed commands with
`disk-nix apply --execute` during activation. This requires
`failOnBlocked = true`.

- `boot`: run the same service-backed policy path as install mode

Boot mode is ordered after local filesystems and udev settle and before
`multi-user.target`. It is intended for boot-time refresh or repair workflows
that still use explicit apply-policy gates.

- `install`: run the same service-backed policy path as activation mode

Install mode skips activation-mode's extra destructive-action assertion. It is
intended for installer or image-build workflows where destructive provisioning
is explicit in the apply policy and confirmation gates.

## Policy

Mutation policy should remain explicit:

- `allowDestructive`
- `allowFormat`
- `allowShrink`
- `allowPotentialDataLoss`
- `allowGrow`
- `allowOffline`
- `allowPropertyChanges`
- `allowDeviceReplacement`
- `allowRebalance`
- `requireBackup`
- `backupVerified`
- `requireConfirmation`
- `confirmation`
- `requireConfirmationFile`
- `probeCurrent`
- `failOnBlocked`
- `execute`
- `scriptOut`
- `reportOut`
- `receiptOut`

`requireBackup` and `requireConfirmation` are additional safety gates for high-risk actions. `allowPotentialDataLoss` is the explicit opt-in for reviewed rollback, shrink, and device-removal workflows, and backup or confirmation requirements still apply when enabled. `requireConfirmationFile` stores the expected file path in the generated policy; `disk-nix apply` only treats it as confirmed when the file contains a standalone line equal to `disk-nix confirm`.

`failOnBlocked` defaults to true. When false, activation and install modes keep writing the same report data but use `disk-nix validate`, which exits successfully even when policy blocks planned actions. `execute` defaults to false. When true, activation and install modes run `disk-nix apply --execute` after policy validation and command-readiness checks pass.

The module requires `failOnBlocked = true` for this mode because `disk-nix validate` is report-only. `scriptOut` must be an absolute path. The apply service creates its parent directory before asking the CLI to write the review script; script generation still refuses policy-blocked or graph-conflicting plans so activation artifacts do not imply a runnable order where none has been proven.

`reportOut` must also be an absolute path. The apply service creates its parent directory before asking the CLI to write the JSON apply report. `receiptOut` must also be an absolute path. The apply service creates its parent directory before asking the CLI to write the receipt envelope containing the report and invocation metadata.
