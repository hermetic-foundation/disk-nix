fn print_topology_summary(
    output: &mut impl Write,
    result: &disk_nix_probe::ProbeResult,
) -> io::Result<()> {
    writeln!(output, "Storage topology probe")?;
    writeln!(output, "nodes: {}", result.graph.nodes.len())?;
    writeln!(output, "edges: {}", result.graph.edges.len())?;
    writeln!(output)?;
    print_probe_reports(output, &result.reports)?;

    Ok(())
}

fn collect_probe_preflight_environment() -> ProbePreflightEnvironment {
    let os_release = fs::read_to_string("/etc/os-release")
        .ok()
        .map(|contents| parse_os_release(&contents))
        .unwrap_or_default();
    ProbePreflightEnvironment {
        os_id: os_release
            .iter()
            .find(|(key, _)| key == "ID")
            .map(|(_, value)| value.clone()),
        os_version_id: os_release
            .iter()
            .find(|(key, _)| key == "VERSION_ID")
            .map(|(_, value)| value.clone()),
        os_pretty_name: os_release
            .iter()
            .find(|(key, _)| key == "PRETTY_NAME")
            .map(|(_, value)| value.clone()),
        kernel_release: command_stdout_first_line("uname", &["-r"]).ok(),
        effective_uid: command_stdout_first_line("id", &["-u"]).ok(),
        tool_versions: storage_tool_version_reports(),
    }
}

fn parse_os_release(contents: &str) -> Vec<(String, String)> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.to_string(), unquote_os_release_value(value)))
        })
        .collect()
}

fn unquote_os_release_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        value.to_string()
    }
}

fn storage_tool_version_reports() -> Vec<ToolVersionReport> {
    [
        ("lsblk", &["--version"][..]),
        ("blkid", &["--version"][..]),
        ("findmnt", &["--version"][..]),
        ("parted", &["--version"][..]),
        ("smartctl", &["--version"][..]),
        ("cryptsetup", &["--version"][..]),
        ("dmsetup", &["version"][..]),
        ("lvm", &["version"][..]),
        ("vdo", &["--version"][..]),
        ("zpool", &["--version"][..]),
        ("zfs", &["--version"][..]),
        ("btrfs", &["--version"][..]),
        ("bcachefs", &["version"][..]),
        ("lsscsi", &["--version"][..]),
        ("iscsiadm", &["--version"][..]),
        ("exportfs", &["--version"][..]),
        ("nfsstat", &["--version"][..]),
        ("mdadm", &["--version"][..]),
        ("multipath", &["-h"][..]),
        ("nvme", &["version"][..]),
    ]
    .into_iter()
    .map(|(tool, args)| storage_tool_version_report(tool, args))
    .collect()
}

fn storage_tool_version_report(tool: &str, args: &[&str]) -> ToolVersionReport {
    match command_stdout_first_line(tool, args) {
        Ok(version) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Available,
            version: Some(version),
            message: None,
        },
        Err(message) if message.contains("not found") || message.contains("No such file") => {
            ToolVersionReport {
                tool: tool.to_string(),
                status: ToolVersionStatus::Unavailable,
                version: None,
                message: Some(message),
            }
        }
        Err(message) => ToolVersionReport {
            tool: tool.to_string(),
            status: ToolVersionStatus::Failed,
            version: None,
            message: Some(message),
        },
    }
}

fn probe_preflight_checks(environment: &ProbePreflightEnvironment) -> ProbePreflightChecks {
    let root = environment.effective_uid.as_deref() == Some("0");
    let missing_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Unavailable)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let failed_tools = environment
        .tool_versions
        .iter()
        .filter(|tool| tool.status == ToolVersionStatus::Failed)
        .map(|tool| tool.tool.clone())
        .collect::<Vec<_>>();
    let mut remediation = Vec::new();
    if !root {
        remediation.push(
            "run probe-status with privileges when adapter metadata requires root-only kernel or device access"
                .to_string(),
        );
    }
    if !missing_tools.is_empty() {
        remediation.push(format!(
            "install or expose missing storage tool(s): {}",
            missing_tools.join(", ")
        ));
        remediation.push(
            "on NixOS, add the required storage packages to environment.systemPackages or services.disk-nix.toolPackages"
                .to_string(),
        );
    }
    if !failed_tools.is_empty() {
        remediation.push(format!(
            "rerun failed storage tool version probe(s) manually with stderr captured: {}",
            failed_tools.join(", ")
        ));
    }
    let status = if root && missing_tools.is_empty() && failed_tools.is_empty() {
        ProbePreflightCheckStatus::Ready
    } else {
        ProbePreflightCheckStatus::Degraded
    };

    ProbePreflightChecks {
        status,
        root,
        unavailable_tool_count: missing_tools.len(),
        failed_tool_count: failed_tools.len(),
        missing_tools,
        failed_tools,
        adapter_remediation: preflight_adapter_remediation(),
        remediation,
    }
}

fn preflight_adapter_remediation() -> Vec<ProbeAdapterRemediation> {
    [
        "lsblk",
        "blkid",
        "findmnt",
        "udev",
        "parted",
        "smartctl",
        "ext",
        "xfs",
        "btrfs",
        "bcachefs",
        "bcache",
        "cryptsetup",
        "dmsetup",
        "lvm",
        "vdo",
        "vdostats",
        "vdostats-verbose",
        "mdraid",
        "mdadm-scan",
        "mdadm-examine",
        "multipath",
        "nfs",
        "nfs-exports",
        "iscsi",
        "iscsi-nodes",
        "lsscsi",
        "nvme",
        "nvme-list-subsys",
        "nvme-smart-log",
        "nvme-id-ctrl",
        "nvme-id-ns",
        "loop",
        "swaps",
        "zramctl",
        "zfs",
    ]
    .into_iter()
    .map(adapter_remediation)
    .collect()
}

fn command_stdout_first_line(command: &str, args: &[&str]) -> Result<String, String> {
    match ProcessCommand::new(command).args(args).output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stdout
                .lines()
                .chain(stderr.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("");
            if line.is_empty() {
                Err(format!("{command} {:?} returned no version text", args))
            } else {
                Ok(line.to_string())
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("command returned a non-zero status");
            Err(format!(
                "{command} {:?} failed with status {}: {detail}",
                args, output.status
            ))
        }
        Err(error) => Err(format!("{command} not found or failed to run: {error}")),
    }
}

fn print_probe_preflight_environment(
    output: &mut impl Write,
    environment: &ProbePreflightEnvironment,
) -> io::Result<()> {
    writeln!(output, "Preflight environment:")?;
    writeln!(
        output,
        "  os: {}",
        environment.os_pretty_name.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-id: {}",
        environment.os_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  os-version-id: {}",
        environment.os_version_id.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  kernel: {}",
        environment.kernel_release.as_deref().unwrap_or("-")
    )?;
    writeln!(
        output,
        "  effective-uid: {}",
        environment.effective_uid.as_deref().unwrap_or("-")
    )?;
    writeln!(output, "  storage tools:")?;
    for tool in &environment.tool_versions {
        let status = match tool.status {
            ToolVersionStatus::Available => "available",
            ToolVersionStatus::Unavailable => "unavailable",
            ToolVersionStatus::Failed => "failed",
        };
        let detail = tool
            .version
            .as_deref()
            .or(tool.message.as_deref())
            .unwrap_or("-");
        writeln!(output, "    {:<12} {:<12} {}", tool.tool, status, detail)?;
    }
    writeln!(output)?;
    Ok(())
}

fn print_probe_preflight_checks(
    output: &mut impl Write,
    checks: &ProbePreflightChecks,
) -> io::Result<()> {
    let status = match checks.status {
        ProbePreflightCheckStatus::Ready => "ready",
        ProbePreflightCheckStatus::Degraded => "degraded",
    };
    writeln!(output, "Preflight checks:")?;
    writeln!(output, "  status: {status}")?;
    writeln!(output, "  root: {}", checks.root)?;
    writeln!(
        output,
        "  unavailable-tools: {}",
        checks.unavailable_tool_count
    )?;
    writeln!(output, "  failed-tools: {}", checks.failed_tool_count)?;
    if !checks.missing_tools.is_empty() {
        writeln!(
            output,
            "  missing-tools: {}",
            checks.missing_tools.join(", ")
        )?;
    }
    if !checks.failed_tools.is_empty() {
        writeln!(
            output,
            "  failed-tool-names: {}",
            checks.failed_tools.join(", ")
        )?;
    }
    for remediation in &checks.remediation {
        writeln!(output, "    remediation: {remediation}")?;
    }
    writeln!(output)?;
    Ok(())
}

fn print_probe_reports(
    output: &mut impl Write,
    reports: &[disk_nix_probe::ProbeReport],
) -> io::Result<()> {
    writeln!(output, "Adapters:")?;

    for report in reports {
        let status = match report.status {
            ProbeStatus::Available => "available",
            ProbeStatus::Unavailable => "unavailable",
            ProbeStatus::Partial => "partial",
            ProbeStatus::Failed => "failed",
        };
        let category = match report.category() {
            ProbeIssueCategory::None => "none",
            ProbeIssueCategory::MissingTool => "missing-tool",
            ProbeIssueCategory::PermissionDenied => "permission-denied",
            ProbeIssueCategory::CommandFailed => "command-failed",
            ProbeIssueCategory::ParseFailed => "parse-failed",
            ProbeIssueCategory::InaccessibleData => "inaccessible-data",
        };

        if let Some(message) = &report.message {
            writeln!(
                output,
                "  {:<12} {:<12} {:<18} {}",
                report.adapter, status, category, message
            )?;
        } else {
            writeln!(
                output,
                "  {:<12} {:<12} {}",
                report.adapter, status, category
            )?;
        }
        for remediation in report.remediation() {
            writeln!(output, "    remediation: {remediation}")?;
        }
    }

    Ok(())
}
