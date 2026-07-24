fn install_zfs_root_spec(options: &InstallZfsRootOptions) -> Value {
    let part_prefix = options
        .part_prefix
        .clone()
        .unwrap_or_else(|| format!("{}-part", options.disk));
    let boot_partition = format!("{part_prefix}1");
    let swap_partition = format!("{part_prefix}2");
    let zfs_partition = format!("{part_prefix}3");
    let root_dataset = options.root_dataset.as_str();
    let home_dataset = format!("{root_dataset}/home");
    let nix_dataset = format!("{root_dataset}/nix");
    let var_dataset = format!("{root_dataset}/var");
    let log_dataset = format!("{root_dataset}/log");

    let mut root_properties = serde_json::json!({
        "acltype": "posixacl",
        "atime": "off",
        "compression": "zstd",
        "mountpoint": "legacy",
        "xattr": "sa"
    });
    if options.encrypt {
        let properties = root_properties
            .as_object_mut()
            .expect("root dataset properties are an object");
        properties.insert(
            "encryption".to_string(),
            Value::String("aes-256-gcm".to_string()),
        );
        properties.insert(
            "keyformat".to_string(),
            Value::String("passphrase".to_string()),
        );
        properties.insert(
            "keylocation".to_string(),
            Value::String("prompt".to_string()),
        );
    }

    let child_properties = serde_json::json!({
        "acltype": "posixacl",
        "atime": "off",
        "compression": "zstd",
        "mountpoint": "legacy",
        "xattr": "sa"
    });

    serde_json::json!({
        "apply": {
            "allowDestructive": true,
            "allowFormat": true,
            "allowOffline": true,
            "mode": "install"
        },
        "version": SUPPORTED_SPEC_VERSION,
        "install": {
            "kind": "nixos-zfs-root",
            "targetDefault": "/mnt",
            "boot": {
                "device": format!("/dev/disk/by-label/{}", options.boot_label),
                "fallbackDevice": boot_partition,
                "mountpoint": "/boot"
            },
            "swap": {
                "device": format!("/dev/disk/by-label/{}", options.swap_label),
                "fallbackDevice": swap_partition
            },
            "zfs": {
                "pool": options.pool,
                "rootDataset": root_dataset,
                "loadKeyDataset": if options.encrypt { Some(root_dataset) } else { None::<&str> },
                "datasets": [
                    { "dataset": root_dataset, "mountpoint": "/" },
                    { "dataset": home_dataset, "mountpoint": "/home" },
                    { "dataset": nix_dataset, "mountpoint": "/nix" },
                    { "dataset": var_dataset, "mountpoint": "/var" },
                    { "dataset": log_dataset, "mountpoint": "/var/log" }
                ]
            }
        },
        "disks": {
            options.disk.as_str(): {
                "operation": "create",
                "partitionType": "gpt"
            }
        },
        "partitions": {
            format!("{}-esp", options.disk): {
                "operation": "create",
                "device": options.disk,
                "name": options.boot_label,
                "partitionNumber": "1",
                "partitionType": "primary",
                "start": options.efi_start,
                "end": options.efi_end,
                "target": boot_partition,
                "metadata": { "gptType": "EF00" }
            },
            format!("{}-swap", options.disk): {
                "operation": "create",
                "device": options.disk,
                "name": options.swap_label,
                "partitionNumber": "2",
                "partitionType": "primary",
                "start": options.swap_start,
                "end": options.swap_end,
                "target": swap_partition,
                "metadata": { "gptType": "8200" }
            },
            format!("{}-zfs", options.disk): {
                "operation": "create",
                "device": options.disk,
                "name": "zfs",
                "partitionNumber": "3",
                "partitionType": "primary",
                "start": options.zfs_start,
                "end": "100%",
                "target": zfs_partition,
                "metadata": { "gptType": "BF01" }
            }
        },
        "filesystems": {
            "boot": {
                "operation": "format",
                "device": boot_partition,
                "fsType": "vfat",
                "mountpoint": "/boot",
                "properties": { "label": options.boot_label },
                "preserveData": false
            }
        },
        "swaps": {
            "disk": {
                "operation": "format",
                "device": swap_partition,
                "properties": { "label": options.swap_label },
                "preserveData": false
            }
        },
        "pools": {
            options.pool.as_str(): {
                "operation": "create",
                "devices": [zfs_partition],
                "properties": {
                    "ashift": "12",
                    "autotrim": "on",
                    "mountpoint": "none"
                }
            }
        },
        "datasets": {
            root_dataset: {
                "operation": "create",
                "properties": root_properties
            },
            home_dataset: {
                "operation": "create",
                "properties": child_properties
            },
            nix_dataset: {
                "operation": "create",
                "properties": child_properties
            },
            var_dataset: {
                "operation": "create",
                "properties": child_properties
            },
            log_dataset: {
                "operation": "create",
                "properties": child_properties
            }
        }
    })
}

#[derive(Debug)]
struct InstallZfsRootOptions {
    disk: String,
    pool: String,
    root_dataset: String,
    boot_label: String,
    swap_label: String,
    efi_start: String,
    efi_end: String,
    swap_start: String,
    swap_end: String,
    zfs_start: String,
    part_prefix: Option<String>,
    encrypt: bool,
}

fn validate_install_zfs_root_options(options: &InstallZfsRootOptions) -> Result<(), AppError> {
    validate_install_label("boot label", &options.boot_label, 11)?;
    validate_install_label("swap label", &options.swap_label, 16)?;
    Ok(())
}

fn validate_install_label(name: &str, label: &str, max_bytes: usize) -> Result<(), AppError> {
    if label.is_empty() {
        return Err(AppError::Message(format!("{name} must not be empty")));
    }
    if label.len() > max_bytes {
        return Err(AppError::Message(format!(
            "{name} {label:?} is too long; maximum is {max_bytes} bytes"
        )));
    }
    if label.contains('/') || label.contains('\0') {
        return Err(AppError::Message(format!(
            "{name} {label:?} cannot contain '/' or NUL because install handoff uses /dev/disk/by-label"
        )));
    }
    Ok(())
}

fn write_install_template(path: &str, spec: &Value) -> Result<(), AppError> {
    let mut json =
        serde_json::to_string_pretty(spec).map_err(|error| AppError::Message(error.to_string()))?;
    json.push('\n');
    std::fs::write(path, json)?;
    Ok(())
}

fn install_mount_script_from_spec_path(spec_path: &str, target: &str) -> Result<String, AppError> {
    let bytes = std::fs::read(spec_path)?;
    plan_from_json_bytes(&bytes)
        .map_err(|error| AppError::Message(format!("failed to parse {spec_path}: {error}")))?;
    let spec: Value = serde_json::from_slice(&bytes)
        .map_err(|error| AppError::Message(format!("failed to parse {spec_path}: {error}")))?;
    install_mount_script_from_spec(&spec, target)
}

fn install_mount_script_from_spec(spec: &Value, target: &str) -> Result<String, AppError> {
    let install = spec
        .get("install")
        .or_else(|| spec.get("spec").and_then(|spec| spec.get("install")))
        .ok_or_else(|| AppError::Message("install metadata is missing from spec".to_string()))?;
    let kind = install
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Message("install.kind is missing from spec".to_string()))?;
    match kind {
        "nixos-zfs-root" => nixos_zfs_root_mount_script(install, target),
        other => Err(AppError::Message(format!(
            "unsupported install.kind {other}; supported kind is nixos-zfs-root"
        ))),
    }
}

fn nixos_zfs_root_mount_script(install: &Value, target: &str) -> Result<String, AppError> {
    let zfs = install
        .get("zfs")
        .ok_or_else(|| AppError::Message("install.zfs is missing from spec".to_string()))?;
    let pool = required_install_str(zfs, "pool")?;
    let boot_device = install
        .get("boot")
        .and_then(|boot| boot.get("device"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Message("install.boot.device is missing from spec".to_string()))?;
    let boot_fallback_device = install
        .get("boot")
        .and_then(|boot| boot.get("fallbackDevice"))
        .and_then(Value::as_str);
    let swap_device = install
        .get("swap")
        .and_then(|swap| swap.get("device"))
        .and_then(Value::as_str);
    let swap_fallback_device = install
        .get("swap")
        .and_then(|swap| swap.get("fallbackDevice"))
        .and_then(Value::as_str);
    let load_key_dataset = zfs.get("loadKeyDataset").and_then(Value::as_str);
    let datasets = zfs
        .get("datasets")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::Message("install.zfs.datasets is missing from spec".to_string()))?;
    let root_dataset = datasets
        .iter()
        .find(|dataset| {
            dataset
                .get("mountpoint")
                .and_then(Value::as_str)
                .is_some_and(|mountpoint| mountpoint == "/")
        })
        .and_then(|dataset| dataset.get("dataset").and_then(Value::as_str))
        .ok_or_else(|| {
            AppError::Message("install.zfs.datasets must include mountpoint /".to_string())
        })?;

    let mut lines = vec![
        "#!/usr/bin/env bash".to_string(),
        "set -euo pipefail".to_string(),
        String::new(),
        format!("target={}", shell_quote(target)),
        String::new(),
        format!("zpool export {}", shell_quote(pool)),
        format!("zpool import -R \"$target\" {}", shell_quote(pool)),
    ];
    if let Some(load_key_dataset) = load_key_dataset {
        lines.push(format!("zfs load-key {}", shell_quote(load_key_dataset)));
    }
    lines.extend([
        "mkdir -p \"$target\"".to_string(),
        format!(
            "mount -t zfs {} \"$target\"",
            shell_quote(root_dataset)
        ),
    ]);

    for dataset in datasets {
        let dataset_name = required_install_str(dataset, "dataset")?;
        let mountpoint = required_install_str(dataset, "mountpoint")?;
        if mountpoint == "/" {
            continue;
        }
        let relative_mountpoint = mountpoint.trim_start_matches('/');
        lines.push(format!("mkdir -p \"$target/{}\"", shell_escape_double(relative_mountpoint)));
        lines.push(format!(
            "mount -t zfs {} \"$target/{}\"",
            shell_quote(dataset_name),
            shell_escape_double(relative_mountpoint)
        ));
    }

    lines.push("mkdir -p \"$target/boot\"".to_string());
    lines.push("udevadm trigger --subsystem-match=block --action=change".to_string());
    lines.push("udevadm settle".to_string());
    lines.extend(install_device_command_with_fallback(
        "mount",
        boot_device,
        boot_fallback_device,
        " \"$target/boot\"",
    ));
    if let Some(swap_device) = swap_device {
        lines.push("udevadm trigger --subsystem-match=block --action=change".to_string());
        lines.push("udevadm settle".to_string());
        lines.extend(install_device_command_with_fallback(
            "swapon",
            swap_device,
            swap_fallback_device,
            "",
        ));
    }
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn nixos_install_script_from_spec_path(
    spec_path: &str,
    target: &str,
    flake: &str,
) -> Result<String, AppError> {
    let mut script = install_mount_script_from_spec_path(spec_path, target)?;
    script.push_str(&format!(
        "nixos-install --root \"$target\" --flake {}\n",
        shell_quote(flake)
    ));
    Ok(script)
}

fn write_install_script(path: &str, script: &str) -> Result<(), AppError> {
    std::fs::write(path, script)?;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

fn execute_install_script(script: &str) -> Result<(), AppError> {
    let status = ProcessCommand::new("bash").arg("-c").arg(script).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::Message(format!(
            "install script failed with status {status}"
        )))
    }
}

fn required_install_str<'a>(value: &'a Value, key: &str) -> Result<&'a str, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Message(format!("install metadata field {key} is missing")))
}

fn install_device_command_with_fallback(
    command: &str,
    primary_device: &str,
    fallback_device: Option<&str>,
    suffix: &str,
) -> Vec<String> {
    let Some(fallback_device) = fallback_device.filter(|device| *device != primary_device) else {
        return vec![format!(
            "{} {}{}",
            command,
            shell_quote(primary_device),
            suffix
        )];
    };
    vec![
        format!("if [[ -e {} ]]; then", shell_quote(primary_device)),
        format!("  {} {}{}", command, shell_quote(primary_device), suffix),
        format!("elif [[ -e {} ]]; then", shell_quote(fallback_device)),
        format!("  {} {}{}", command, shell_quote(fallback_device), suffix),
        "else".to_string(),
        format!("  {} {}{}", command, shell_quote(primary_device), suffix),
        "fi".to_string(),
    ]
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn shell_escape_double(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
