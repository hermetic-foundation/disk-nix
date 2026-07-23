{
  pkgs,
  nixosModuleExecuteTest,
  nixosModuleHandoffAutoImportTest,
  nixosModuleBootModeTest,
  nixosModuleInstallModeTest,
  ...
}:

{
  nixosModuleExecute =
    pkgs.runCommand "disk-nix-nixos-module-execute-check" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        spec=${nixosModuleExecuteTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '
          .apply.mode == "activation"
          and .apply.failOnBlocked == true
          and .apply.probeCurrent == true
          and has("apply")
          and (.apply | has("execute") | not)
        ' "$spec"
        applyScript='${nixosModuleExecuteTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$applyScript"
        grep -- '--execute' "$applyScript"
        grep -- '--probe-current' "$applyScript"
        grep -- '--script-out' "$applyScript"
        grep -- '/run/disk-nix/execute.sh' "$applyScript"
        grep -- '--report-out' "$applyScript"
        grep -- '/run/disk-nix/execute-report.json' "$applyScript"
        grep -- '--receipt-out' "$applyScript"
        grep -- '/run/disk-nix/execute-receipt.json' "$applyScript"
        touch "$out"
      '';
  nixosModuleHandoffAutoImport =
    pkgs.runCommand "disk-nix-nixos-module-handoff-auto-import-check"
      { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        spec=${nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '
          .apply.mode == "activation"
          and .apply.failOnBlocked == true
          and (.apply | has("execute") | not)
          and (.apply | has("declarativeHandoff") | not)
        ' "$spec"
        steadyState=${
          pkgs.lib.escapeShellArg (
            builtins.readFile
              nixosModuleHandoffAutoImportTest.config.environment.etc."disk-nix/steady-state.json".source
          )
        }
        printf '%s\n' "$steadyState" > steady-state
        jq -e '
          .declarativeHandoff.autoImport.enabled == true
          and .declarativeHandoff.autoImport.configurationPath == "/etc/nixos/storage.nix"
          and .declarativeHandoff.autoImport.backupDirectory == "/var/backups/disk-nix-handoff"
        ' steady-state
        applyScript='${nixosModuleHandoffAutoImportTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$applyScript"
        grep -- '--execute' "$applyScript"
        grep -F -- 'config_path=/etc/nixos/storage.nix' "$applyScript"
        grep -F -- 'backup_dir=/var/backups/disk-nix-handoff' "$applyScript"
        grep -F -- 'handoff_module=/etc/disk-nix/declarative-handoff.nix' "$applyScript"
        grep -F -- 'import_patch=/etc/disk-nix/declarative-handoff-import.patch' "$applyScript"
        grep -F -- 'grep -F -q "$handoff_module" "$config_path"' "$applyScript"
        grep -F -- 'cp --preserve=mode,ownership,timestamps "$config_path" "$backup_path"' "$applyScript"
        grep -F -- 'patch --forward --backup --input="$import_patch" "$config_path"' "$applyScript"
        touch "$out"
      '';
  nixosModuleApplyModes =
    pkgs.runCommand "disk-nix-nixos-module-apply-modes-check" { nativeBuildInputs = [ pkgs.jq ]; }
      ''
        bootWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.warnings)}
        installWarnings=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.warnings)}
        ! printf '%s\n' "$bootWarnings" | grep -- 'apply.mode = \\"boot\\" is reserved'
        ! printf '%s\n' "$installWarnings" | grep -- 'apply.mode = \\"install\\" is reserved'
        bootSpec=${nixosModuleBootModeTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '.apply.mode == "boot"' "$bootSpec"
        bootScript='${nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$bootScript"
        bootWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
        printf '%s\n' "$bootWantedBy" | jq -e 'index("multi-user.target") != null'
        bootWants=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.wants)}
        printf '%s\n' "$bootWants" | jq -e 'index("systemd-udev-settle.service") != null'
        bootAfter=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.after)}
        printf '%s\n' "$bootAfter" | jq -e 'index("local-fs.target") != null and index("systemd-udev-settle.service") != null'
        bootBefore=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleBootModeTest.config.systemd.services.disk-nix-plan.before)}
        printf '%s\n' "$bootBefore" | jq -e 'index("multi-user.target") != null'
        installSpec=${nixosModuleInstallModeTest.config.environment.etc."disk-nix/spec.json".source}
        jq -e '.apply.mode == "install"' "$installSpec"
        installScript='${nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.serviceConfig.ExecStart}'
        grep -- 'apply' "$installScript"
        installWantedBy=${pkgs.lib.escapeShellArg (builtins.toJSON nixosModuleInstallModeTest.config.systemd.services.disk-nix-plan.wantedBy)}
        printf '%s\n' "$installWantedBy" | jq -e 'index("multi-user.target") != null'
        touch "$out"
      '';
}
