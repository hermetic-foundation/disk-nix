fn run(cli: Cli, output: &mut impl Write) -> Result<(), AppError> {
    match cli.command {
        Command::Topology { json: false } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            print_topology_summary(output, &result)?;
            Ok(())
        }
        Command::Topology { json: true } => {
            let graph = collect_graph()?;
            writeln!(
                output,
                "{}",
                graph
                    .to_json()
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::ProbeStatus { json, preflight } => {
            let probe = LinuxProbe::new();
            let result = probe
                .collect()
                .map_err(|error| AppError::Message(error.to_string()))?;
            if json {
                if preflight {
                    let environment = collect_probe_preflight_environment();
                    let preflight_checks = probe_preflight_checks(&environment);
                    let report = ProbeStatusPreflightReport {
                        environment,
                        preflight_checks,
                        reports: result.reports,
                    };
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&report)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                } else {
                    writeln!(
                        output,
                        "{}",
                        serde_json::to_string_pretty(&result.reports)
                            .map_err(|error| AppError::Message(error.to_string()))?
                    )?;
                }
            } else if preflight {
                let environment = collect_probe_preflight_environment();
                let preflight_checks = probe_preflight_checks(&environment);
                print_probe_preflight_environment(output, &environment)?;
                print_probe_preflight_checks(output, &preflight_checks)?;
                print_probe_reports(output, &result.reports)?;
            } else {
                print_probe_reports(output, &result.reports)?;
            }
            Ok(())
        }
        Command::Capabilities { json } => {
            let capabilities = default_capabilities();
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&capabilities)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                for capability in capabilities {
                    writeln!(
                        output,
                        "{:?} {:?} {:?}",
                        capability.node_kind, capability.operation, capability.risk
                    )?;
                }
            }
            Ok(())
        }
        Command::Devices { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_device_node)?;
            } else {
                print_devices(output, &graph)?;
            }
            Ok(())
        }
        Command::Partitions { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_partition_node)?;
            } else {
                print_partitions(output, &graph)?;
            }
            Ok(())
        }
        Command::Filesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_filesystem_node)?;
            } else {
                print_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::ComplexFilesystems { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_complex_filesystem_node)?;
            } else {
                print_complex_filesystems(output, &graph)?;
            }
            Ok(())
        }
        Command::Btrfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_btrfs_node)?;
            } else {
                print_btrfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Bcachefs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_bcachefs_node)?;
            } else {
                print_bcachefs(output, &graph)?;
            }
            Ok(())
        }
        Command::Zfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zfs_node)?;
            } else {
                print_zfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Volumes { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_volume_node)?;
            } else {
                print_volumes(output, &graph)?;
            }
            Ok(())
        }
        Command::Pools { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_pool_node)?;
            } else {
                print_pools(output, &graph)?;
            }
            Ok(())
        }
        Command::Snapshots { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_snapshot_node)?;
            } else {
                print_snapshots(output, &graph)?;
            }
            Ok(())
        }
        Command::Mappings { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mapping_node)?;
            } else {
                print_mappings(output, &graph)?;
            }
            Ok(())
        }
        Command::Dm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_dm_node)?;
            } else {
                print_dm(output, &graph)?;
            }
            Ok(())
        }
        Command::Encryption { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_encryption_node)?;
            } else {
                print_encryption(output, &graph)?;
            }
            Ok(())
        }
        Command::Cache { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_cache_node)?;
            } else {
                print_cache(output, &graph)?;
            }
            Ok(())
        }
        Command::Lvm { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lvm_node)?;
            } else {
                print_lvm(output, &graph)?;
            }
            Ok(())
        }
        Command::Vdo { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_vdo_node)?;
            } else {
                print_vdo(output, &graph)?;
            }
            Ok(())
        }
        Command::Multipath { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_multipath_node)?;
            } else {
                print_multipath(output, &graph)?;
            }
            Ok(())
        }
        Command::Nvme { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nvme_node)?;
            } else {
                print_nvme(output, &graph)?;
            }
            Ok(())
        }
        Command::Raid { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_raid_node)?;
            } else {
                print_raid(output, &graph)?;
            }
            Ok(())
        }
        Command::Loop { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_loop_node)?;
            } else {
                print_loop(output, &graph)?;
            }
            Ok(())
        }
        Command::BackingFiles { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_backing_file_node)?;
            } else {
                print_backing_files(output, &graph)?;
            }
            Ok(())
        }
        Command::Swap { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_swap_node)?;
            } else {
                print_swap(output, &graph)?;
            }
            Ok(())
        }
        Command::Zram { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_zram_node)?;
            } else {
                print_zram(output, &graph)?;
            }
            Ok(())
        }
        Command::Iscsi { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_iscsi_node)?;
            } else {
                print_iscsi(output, &graph)?;
            }
            Ok(())
        }
        Command::Luns { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_lun_node)?;
            } else {
                print_luns(output, &graph)?;
            }
            Ok(())
        }
        Command::Nfs { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_nfs_node)?;
            } else {
                print_nfs(output, &graph)?;
            }
            Ok(())
        }
        Command::Mounts { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_mount_node)?;
            } else {
                print_mounts(output, &graph)?;
            }
            Ok(())
        }
        Command::NetworkStorage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, is_network_storage_node)?;
            } else {
                print_network_storage(output, &graph)?;
            }
            Ok(())
        }
        Command::Ids { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_identity)?;
            } else {
                print_ids(output, &graph)?;
            }
            Ok(())
        }
        Command::Usage { json } => {
            let graph = collect_graph()?;
            if json {
                print_filtered_json(output, &graph, has_capacity_or_usage)?;
            } else {
                print_usage(output, &graph)?;
            }
            Ok(())
        }
        Command::Inspect { query, depth, json } => {
            let graph = collect_graph()?;
            if json {
                print_inspect_json(output, &graph, &query, depth)?;
            } else {
                print_inspect(output, &graph, &query, depth)?;
            }
            Ok(())
        }
        Command::Plan {
            spec,
            probe_current,
            json,
        } => {
            let bytes = std::fs::read(&spec)?;
            let mut plan = plan_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to parse {spec}: {error}")))?;
            if probe_current {
                plan = compare_plan_with_topology(plan, &collect_graph()?);
            }
            if json {
                writeln!(
                    output,
                    "{}",
                    plan.to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_plan(output, &plan)?;
            }
            Ok(())
        }
        Command::Apply {
            spec,
            probe_current,
            execute,
            script_out,
            report_out,
            receipt_out,
            json,
        } => {
            let mode = if execute {
                ExecutionMode::Execute
            } else {
                ExecutionMode::DryRun
            };
            let report = prepare_apply_report(&spec, probe_current, mode)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "apply",
                        &spec,
                        probe_current,
                        execute,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, execute)?;
            }

            if report.status == ExecutionStatus::Blocked {
                return Err(AppError::Message(format!(
                    "apply policy blocked {} action(s)",
                    report.apply.blocked_count
                )));
            }
            if matches!(
                report.status,
                ExecutionStatus::NotReady | ExecutionStatus::Failed
            ) {
                return Err(AppError::Message(report.messages.join("; ")));
            }

            Ok(())
        }
        Command::Validate {
            spec,
            probe_current,
            script_out,
            report_out,
            receipt_out,
            json,
        } => {
            let report = prepare_apply_report(&spec, probe_current, ExecutionMode::DryRun)?;
            if let Some(report_out) = report_out.as_deref() {
                write_execution_report(report_out, &report)?;
            }
            if let Some(receipt_out) = receipt_out.as_deref() {
                write_apply_receipt(
                    receipt_out,
                    apply_receipt(
                        "validate",
                        &spec,
                        probe_current,
                        false,
                        current_unix_seconds()?,
                        &report,
                    ),
                )?;
            }
            if let Some(script_out) = script_out.as_deref() {
                write_execution_script(script_out, &report)?;
            }

            if json {
                writeln!(
                    output,
                    "{}",
                    report
                        .to_json()
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_execution_report(output, &report, false)?;
            }

            Ok(())
        }
        Command::Install { command } => match command {
            InstallCommand::Template { command } => match command {
                InstallTemplateCommand::ZfsRoot {
                    disk,
                    out,
                    pool,
                    root_dataset,
                    boot_label,
                    swap_label,
                    efi_start,
                    efi_end,
                    swap_start,
                    swap_end,
                    zfs_start,
                    part_prefix,
                    encrypt,
                } => {
                    let root_dataset = root_dataset.unwrap_or_else(|| format!("{pool}/root"));
                    let options = InstallZfsRootOptions {
                        disk,
                        pool,
                        root_dataset,
                        boot_label,
                        swap_label,
                        efi_start,
                        efi_end,
                        swap_start,
                        swap_end,
                        zfs_start,
                        part_prefix,
                        encrypt,
                    };
                    validate_install_zfs_root_options(&options)?;
                    let spec = install_zfs_root_spec(&options);
                    write_install_template(&out, &spec)?;
                    writeln!(output, "wrote {out}")?;
                    Ok(())
                }
            },
            InstallCommand::Mount {
                spec,
                target,
                script_out,
                execute,
            } => {
                let script = install_mount_script_from_spec_path(&spec, &target)?;
                if let Some(script_out) = script_out.as_deref() {
                    write_install_script(script_out, &script)?;
                    writeln!(output, "wrote {script_out}")?;
                } else {
                    write!(output, "{script}")?;
                }
                if execute {
                    execute_install_script(&script)?;
                }
                Ok(())
            }
            InstallCommand::Nixos {
                spec,
                flake,
                target,
                script_out,
                execute,
            } => {
                let script = nixos_install_script_from_spec_path(&spec, &target, &flake)?;
                if let Some(script_out) = script_out.as_deref() {
                    write_install_script(script_out, &script)?;
                    writeln!(output, "wrote {script_out}")?;
                } else {
                    write!(output, "{script}")?;
                }
                if execute {
                    execute_install_script(&script)?;
                }
                Ok(())
            }
        },
        Command::Migrate { spec, json } => {
            let bytes = std::fs::read(&spec)?;
            let report = migration_report_from_json_bytes(&bytes)
                .map_err(|error| AppError::Message(format!("failed to migrate {spec}: {error}")))?;
            if json {
                writeln!(
                    output,
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .map_err(|error| AppError::Message(error.to_string()))?
                )?;
            } else {
                print_migration_report(output, &report)?;
            }
            Ok(())
        }
        Command::Schema => {
            writeln!(
                output,
                "{}",
                serde_json::to_string_pretty(&spec_schema())
                    .map_err(|error| AppError::Message(error.to_string()))?
            )?;
            Ok(())
        }
        Command::Completions { shell } => {
            let mut command = Cli::command();
            generate(shell, &mut command, "disk-nix", output);
            Ok(())
        }
        Command::Manpage => {
            let command = Cli::command();
            Man::new(command).render(output)?;
            Ok(())
        }
    }
}
