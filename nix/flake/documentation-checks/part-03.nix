{
  pkgs,
  self,
  root,
  diskNix,
  integrationDiskoExamples,
  ...
}:

''
  ${pkgs.gnugrep}/bin/grep -q 'shared namespace UUID/NGUID identity' ${
    root + /docs/user/storage-scope.md
  }
  ${pkgs.gnugrep}/bin/grep -q 'nvme_of_mixed_fabric_fixture_preserves_sharing_and_path_churn' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'bbbbbbbb-cccc-dddd-eeee-ffffffffffff' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'uuid: namespace_uuid' ${
    root + /crates/disk-nix-probe/src/nvme/graph.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'Real-world clustered storage fixture coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'DLM/lvmlockd failure fixture' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'split-brain protection refusal' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'clustered_lvm_failure_fixture_preserves_lock_manager_and_split_brain_state' ${
    root + /crates/disk-nix-probe/src/tests/part_03.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'lvm.vg-lock-failure' ${
    root + /crates/disk-nix-probe/src/lvm/volume_groups.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'NFS server/client fixture' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'NFS server/client fixture' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'Real-world server/client NFS fixture coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'client remount drift' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'pNFS layout and' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'nfs_server_client_fixture_merges_mount_usage_and_export_policy' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'nfs.export-option-sec", "krb5p' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'normalizes_referral_pnfs_remount_and_export_reload_fixture' ${
    root + /crates/disk-nix-probe/src/nfs.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'nfs.export-option-pnfs' ${root + /crates/disk-nix-probe/src/nfs.rs}
  ${pkgs.gnugrep}/bin/grep -q 'SAS enclosure fixture' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Real-world hardware enclosure and array fixture coverage' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'vendor LUN metadata' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'SES failure attributes' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'hardware_array_fixture_preserves_ses_failures_and_identity_drift' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'vdisk-prod-77-replaced' ${
    root + /crates/disk-nix-probe/src/tests/part_02.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'LVM-backed VDO fixture' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'stressed VDO fixture' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'vdo_pressure_fixture_preserves_rebuild_policy_and_failure_state' ${
    root + /crates/disk-nix-probe/src/tests/part_03.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'physical-space pressure' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'non-block SES enclosure records' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'LVM-backed VDO fixture' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'active/standby state' ${root + /docs/user/storage-scope.md}
  ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'emulate_write_cache=0' $failureRecoverySources
  ${pkgs.gnugrep}/bin/grep -q 'tgt property updates render' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'provider = "scst"' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'providerCapabilities' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'provider capability contracts' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'target-lun.capacity.expand' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'target_lun_lio_backing_size_command' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'LIO target-side LUN grow has a native reviewed block' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'target_lun_lio_fileio_grow_forces_backstore_resize_before_refresh' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'backstoreType = "fileio"' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'truncate --size <desiredSize> <source>' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'target_lun_tgt_logical_unit_refresh_command' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'tgt target-side LUN grow has a native reviewed refresh path' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Generic target LUN verification plans' ${root + /docs/user/status.md}
  ${pkgs.gnugrep}/bin/grep -q 'target_lun_generic_host_verification_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'arrayId' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'target-lun.array-id.declared' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'read_only_validation' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'RollbackExecutionReport' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_unsafe_sections_and_not_ready_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_missing_tools_before_running_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_recipe_safety_gates' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'filesystem rollback gates' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'block-stack rollback gates' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'advanced-storage rollback gates' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'network-storage rollback gates' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'required_topology_evidence' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'materialize_rollback_topology_evidence' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'materialize_rollback_topology_payloads' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'replay_proven_safe_rollback_recipe_with_topology_payloads' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'topology_payloads' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_evidence_materializes_from_failed_report_and_fresh_probe' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_binds_full_topology_payloads_to_receipt' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_comparison_refusal_reasons' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_refusal_reasons' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_live_use_blocker' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_stale_identity_blocker' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_idempotency_blocker' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_topology_diagnostic_is_data_loss_risk' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_divergent_topology_comparison_before_running_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_risky_topology_diagnostics_before_running_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'topology-already-rolled-back' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_refuses_missing_required_topology_evidence_before_running_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_replay_requires_original_receipt_binding_before_running_commands' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_command_data_loss_risk_reason' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_command_live_use_blocker_reason' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_command_identity_blocker_reason' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollback_command_idempotency_blocker_reason' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'live-use-blocker-metadata' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'ambiguous-stale-identity-metadata' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'idempotency-externally-modified-metadata' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'plausible data-loss command metadata' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay refuses missing required tools' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit filesystem safety gates' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit block-stack safety gates' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit advanced-storage safety' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes emit network-storage safety gates' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'metadata advertises already rolled-back' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'idempotency diagnostics for already satisfied' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'detailed post-failure topology diagnostics report divergent' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'ambiguous rollback points and stale identity data' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'behavior for mounted filesystems' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'topology-aware refusal' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback recipes declare required topology' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'negative tests proving' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'not bound to the failed' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'current topology differs' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'data-loss-prone operations make rollback unsafe' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'Automatic rollback replay can materialize deterministic' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'receiptBinding.topologyPayloads' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'crate-level integration' $checklistSources
  ${pkgs.gnugrep}/bin/grep -q 'proven_rollback_recipe_replays_and_emits_receipt_binding' ${
    root + /crates/disk-nix-exec/tests/rollback_replay.rs
  }
  ${pkgs.gnugrep}/bin/grep -q 'filesystem_remount_failure_emits_proven_safe_rollback_recipe' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'filesystem_property_failure_emits_proven_safe_rollback_recipe' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'filesystem_check_scrub_and_repair_failures_emit_refused_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'block_stack_property_failures_emit_proven_safe_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'block_stack_verification_failures_emit_proven_safe_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'block_stack_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'block_stack_zram_boundary_emits_refused_rollback_recipe' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'advanced_storage_property_failures_emit_proven_safe_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'advanced_storage_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'network_storage_failures_emit_proven_safe_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'network_storage_refused_boundaries_emit_operator_only_rollback_recipes' $execSources
  ${pkgs.gnugrep}/bin/grep -q 'rollbackOptions' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'rollbackValue' ${root + /docs/developer/planning.md}
  ${pkgs.gnugrep}/bin/grep -q 'device-mapper rename verification failures' ${root + /docs/user/cli.md}
  ${pkgs.gnugrep}/bin/grep -q 'Block-stack property declarations use the same' ${
    root + /docs/developer/planning.md
  }
''
