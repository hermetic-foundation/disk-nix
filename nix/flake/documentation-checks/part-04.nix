{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

''
  ${pkgs.gnugrep}/bin/grep -q 'Advanced-storage declarations also use' ${
    root + /docs/developer/planning.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'ZFS snapshot rollback/clone' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'Network-storage declarations also use' ${
    root + /docs/developer/planning.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'Network-storage failures can also produce proven-safe recipes' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses proven-safe recipes when' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'commands whose metadata advertises ambiguous rollback points' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'commands whose metadata advertises active consumers' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses reversible mutation' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'rollbackRecipes' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'requiredTopologyEvidence' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe_with_topology_evidence' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'topology comparison summary already has missing targets' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'open encrypted mappings, active' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'ambiguous rollback points, ambiguous rollback targets' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'Idempotency' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'operatorOnlyHandoff' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'proven-safe reversible rollback' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback has an execution engine' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses' ${
    root + /docs/developer/feature-checklist.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'scstadmin' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'initiatorGroup' ${root + /docs/developer/planning.md}
  runbooks=${root + /docs/user/operator-runbooks.md}
  for runbook in \
    "Device replacement" \
    Rollback \
    "Failed apply recovery" \
    "Degraded arrays and pools" \
    "Shared storage and network storage" \
    "Change record"
  do
    ${pkgs.gnugrep}/bin/grep -q "^## $runbook$" "$runbooks"
  done
  for section in \
    Foundation \
    "Read-only storage awareness" \
    "Planning and apply safety" \
    "Lifecycle operations" \
    "Current-topology reconciliation" \
    "Recovery guidance" \
    "NixOS integration" \
    "Testing and proof" \
    Documentation
  do
    ${pkgs.gnugrep}/bin/grep -q "^## $section$" "$checklist"
  done
''
