{ lib, operationType }:

{
  mode = lib.mkOption {
    type = lib.types.enum [
      "manual"
      "activation"
      "boot"
      "install"
    ];
    default = "manual";
    description = "When disk-nix may perform imperative storage actions.";
  };

  allowDestructive = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Allow destructive storage actions such as wipe, format, or destroy.";
  };

  allowFormat = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Allow formatting filesystems.";
  };

  allowShrink = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Allow shrink operations.";
  };

  allowPotentialDataLoss = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Allow explicitly reviewed potential-data-loss actions such as shrink, rollback, and device removal after any configured backup or confirmation gates pass.";
  };

  allowGrow = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Allow non-destructive grow operations.";
  };

  allowOffline = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Allow storage operations that require offline coordination.";
  };

  allowPropertyChanges = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Allow non-destructive storage property changes.";
  };

  allowDeviceReplacement = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Allow device add, replacement, and removal topology changes.";
  };

  allowRebalance = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Allow pool or filesystem rebalance operations.";
  };

  requireBackup = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Require backupVerified=true for destructive or potential-data-loss actions.";
  };

  backupVerified = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Assert that required backups have been verified before policy validation.";
  };

  requireConfirmation = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Require confirmation=true for high-risk or offline actions.";
  };

  confirmation = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Explicit operator confirmation for policies that require it.";
  };

  requireConfirmationFile = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    description = "Path to an operator-controlled confirmation file. disk-nix apply confirms it only when the file contains a standalone 'disk-nix confirm' line.";
  };

  probeCurrent = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Probe current topology during disk-nix apply-policy validation.";
  };

  failOnBlocked = lib.mkOption {
    type = lib.types.bool;
    default = true;
    description = "Fail the activation service when policy blocks planned actions. When false, activation uses disk-nix validate so blocked actions are reported without failing the unit.";
  };

  execute = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = "Run ready, policy-allowed storage commands during activation with disk-nix apply --execute. The default only validates policy and writes review artifacts.";
  };

  scriptOut = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "/run/disk-nix/apply.sh";
    description = "Write the allowed command and verification plan to this reviewable shell script path during apply-policy validation.";
  };

  reportOut = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "/run/disk-nix/apply-report.json";
    description = "Write the JSON apply-policy report to this path during validation, including blocked policy details before failures are returned.";
  };

  receiptOut = lib.mkOption {
    type = lib.types.nullOr lib.types.str;
    default = null;
    example = "/run/disk-nix/apply-receipt.json";
    description = "Write a JSON apply-policy receipt to this path during validation, binding the report to the invoked command, spec path, probe-current choice, execute choice, and generation timestamp.";
  };

  declarativeHandoff.autoImport = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "After a successful imperative disk-nix apply --execute run, apply the generated declarative-handoff import patch to the configured NixOS configuration file. This is disabled by default because it edits user-owned declarative configuration.";
    };

    configurationPath = lib.mkOption {
      type = lib.types.str;
      default = "/etc/nixos/configuration.nix";
      example = "/etc/nixos/hosts/storage/configuration.nix";
      description = "NixOS configuration file to patch with an import of /etc/disk-nix/declarative-handoff.nix when declarative handoff auto-import is enabled.";
    };

    backupDirectory = lib.mkOption {
      type = lib.types.str;
      default = "/var/backups/disk-nix";
      example = "/persist/backups/disk-nix";
      description = "Directory where disk-nix stores a timestamped copy of the configuration file before applying the declarative handoff import patch.";
    };
  };
}
