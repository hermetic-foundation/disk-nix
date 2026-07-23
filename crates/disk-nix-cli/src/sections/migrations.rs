fn migration_report_from_json_bytes(bytes: &[u8]) -> Result<MigrationReport, AppError> {
    let mut value: Value =
        serde_json::from_slice(bytes).map_err(|error| AppError::Message(error.to_string()))?;
    let source_version = migration_source_version(&value)?;
    let target_version = SUPPORTED_SPEC_VERSION;
    if source_version.is_some_and(|version| version != target_version) {
        return Err(AppError::Message(format!(
            "unsupported disk-nix spec version {}; supported migration target is {target_version}",
            source_version.expect("checked")
        )));
    }

    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let mut applied_mappings = Vec::new();
    apply_legacy_pre_version_mappings(
        &mut value,
        source_version,
        &mut changes,
        &mut applied_mappings,
    )?;
    ensure_object_version(&mut value, "version", target_version, &mut changes)?;
    if let Some(spec) = value.get_mut("spec") {
        ensure_object_version(spec, "spec.version", target_version, &mut changes)?;
    }
    if changes.is_empty() {
        changes.push("spec already declares the current supported contract version".to_string());
    }
    warnings.push(
        "migration does not apply storage mutations; run plan or apply separately after review"
            .to_string(),
    );
    warnings.push(
        "version 1 migration only normalizes metadata and documented legacy pre-versioned field names"
            .to_string(),
    );

    let serialized =
        serde_json::to_vec(&value).map_err(|error| AppError::Message(error.to_string()))?;
    plan_from_json_bytes(&serialized)
        .map_err(|error| AppError::Message(format!("migrated spec is invalid: {error}")))?;

    Ok(MigrationReport {
        source_version,
        target_version,
        migrated: !changes
            .iter()
            .any(|change| change == "spec already declares the current supported contract version"),
        version_migrations: version_migration_contracts(),
        legacy_mappings: legacy_pre_version_mappings(),
        applied_mappings,
        changes,
        warnings,
        spec: value,
    })
}

fn apply_legacy_pre_version_mappings(
    value: &mut Value,
    source_version: Option<u64>,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    if source_version.is_some() {
        return Ok(());
    }
    normalize_legacy_pre_version_container(value, "", changes, applied_mappings)?;
    if let Some(spec) = value.get_mut("spec") {
        normalize_legacy_pre_version_container(spec, "spec.", changes, applied_mappings)?;
    }
    Ok(())
}

fn normalize_legacy_pre_version_container(
    value: &mut Value,
    prefix: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    rename_legacy_field(
        value,
        prefix,
        "fileSystems",
        "filesystems",
        changes,
        applied_mappings,
    )?;
    rename_legacy_field(
        value,
        prefix,
        "swapDevices",
        "swaps",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "luksDevices",
        "luks",
        "devices",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "nfsMounts",
        "nfs",
        "mounts",
        changes,
        applied_mappings,
    )?;
    move_legacy_nested_field(
        value,
        prefix,
        "iscsiSessions",
        "iscsi",
        "sessions",
        changes,
        applied_mappings,
    )?;
    Ok(())
}

fn rename_legacy_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    current: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }
    if object.contains_key(current) {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{current}"
        )));
    }
    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    object.insert(current.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{current}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, current));
    Ok(())
}

fn move_legacy_nested_field(
    value: &mut Value,
    prefix: &str,
    legacy: &str,
    parent: &str,
    child: &str,
    changes: &mut Vec<String>,
    applied_mappings: &mut Vec<LegacyMigrationMapping>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Ok(());
    };
    if !object.contains_key(legacy) {
        return Ok(());
    }

    if object
        .get(parent)
        .and_then(Value::as_object)
        .is_some_and(|parent| parent.contains_key(child))
    {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} conflicts with current field {prefix}{parent}.{child}"
        )));
    }

    let Some(mapped) = object.remove(legacy) else {
        return Ok(());
    };
    let parent_value = object
        .entry(parent.to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    let Value::Object(parent_object) = parent_value else {
        return Err(AppError::Message(format!(
            "legacy field {prefix}{legacy} cannot be mapped because {prefix}{parent} is not an object"
        )));
    };
    parent_object.insert(child.to_string(), mapped);
    changes.push(format!(
        "mapped legacy field {prefix}{legacy} to {prefix}{parent}.{child}"
    ));
    applied_mappings.push(legacy_mapping(prefix, legacy, &format!("{parent}.{child}")));
    Ok(())
}

fn legacy_mapping(prefix: &str, source: &str, target: &str) -> LegacyMigrationMapping {
    LegacyMigrationMapping {
        source: format!("{prefix}{source}"),
        target: format!("{prefix}{target}"),
        scope: if prefix.is_empty() {
            "top-level".to_string()
        } else {
            prefix.trim_end_matches('.').to_string()
        },
    }
}

fn legacy_pre_version_mappings() -> Vec<LegacyMigrationMapping> {
    ["", "spec."]
        .into_iter()
        .flat_map(|prefix| {
            [
                ("fileSystems", "filesystems"),
                ("swapDevices", "swaps"),
                ("luksDevices", "luks.devices"),
                ("nfsMounts", "nfs.mounts"),
                ("iscsiSessions", "iscsi.sessions"),
            ]
            .into_iter()
            .map(move |(source, target)| legacy_mapping(prefix, source, target))
        })
        .collect()
}

fn version_migration_contracts() -> Vec<VersionMigrationContract> {
    vec![
        VersionMigrationContract {
            source_version: None,
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "pre-version legacy aliases to version 1".to_string(),
            field_mappings: legacy_pre_version_mappings(),
            safety_notes: vec![
                "applies only to unversioned documents".to_string(),
                "does not apply storage mutations".to_string(),
                "conflicting legacy and current fields are rejected".to_string(),
            ],
        },
        VersionMigrationContract {
            source_version: Some(SUPPORTED_SPEC_VERSION),
            target_version: SUPPORTED_SPEC_VERSION,
            status: "supported".to_string(),
            mapping_scope: "version 1 metadata normalization".to_string(),
            field_mappings: Vec::new(),
            safety_notes: vec![
                "explicit version 1 documents are validated without legacy alias rewrites"
                    .to_string(),
                "does not apply storage mutations".to_string(),
            ],
        },
    ]
}

fn migration_source_version(value: &Value) -> Result<Option<u64>, AppError> {
    let top_level = optional_version_field(value, "version")?;
    let spec = value
        .get("spec")
        .map(|spec| optional_version_field(spec, "spec.version"))
        .transpose()?
        .flatten();
    if let (Some(top_level), Some(spec)) = (top_level, spec) {
        if top_level != spec {
            return Err(AppError::Message(format!(
                "conflicting disk-nix spec versions: top-level version {top_level}, spec.version {spec}"
            )));
        }
    }
    Ok(top_level.or(spec))
}

fn optional_version_field(value: &Value, location: &str) -> Result<Option<u64>, AppError> {
    let Some(version) = value.get("version") else {
        return Ok(None);
    };
    version.as_u64().map(Some).ok_or_else(|| {
        AppError::Message(format!(
            "disk-nix spec version at {location} must be an integer"
        ))
    })
}

fn ensure_object_version(
    value: &mut Value,
    location: &str,
    target_version: u64,
    changes: &mut Vec<String>,
) -> Result<(), AppError> {
    let Value::Object(object) = value else {
        return Err(AppError::Message(format!(
            "disk-nix spec at {location} must be an object to add version metadata"
        )));
    };
    match object.get("version").and_then(Value::as_u64) {
        Some(version) if version == target_version => Ok(()),
        Some(version) => Err(AppError::Message(format!(
            "unsupported disk-nix spec version {version}; supported migration target is {target_version}"
        ))),
        None => {
            object.insert("version".to_string(), Value::from(target_version));
            changes.push(format!("set {location} to {target_version}"));
            Ok(())
        }
    }
}

fn print_migration_report(output: &mut impl Write, report: &MigrationReport) -> io::Result<()> {
    writeln!(
        output,
        "Migration: {:?} -> {}",
        report.source_version, report.target_version
    )?;
    writeln!(output, "migrated: {}", report.migrated)?;
    writeln!(output, "Changes:")?;
    for change in &report.changes {
        writeln!(output, "- {change}")?;
    }
    writeln!(output, "Version migration contracts:")?;
    for contract in &report.version_migrations {
        writeln!(
            output,
            "- {:?} -> {}: {} ({})",
            contract.source_version,
            contract.target_version,
            contract.status,
            contract.mapping_scope
        )?;
        if contract.field_mappings.is_empty() {
            writeln!(output, "  field mappings: none")?;
        } else {
            writeln!(output, "  field mappings:")?;
            for mapping in &contract.field_mappings {
                writeln!(
                    output,
                    "  - {} -> {} ({})",
                    mapping.source, mapping.target, mapping.scope
                )?;
            }
        }
    }
    writeln!(output, "Legacy mappings:")?;
    for mapping in &report.legacy_mappings {
        writeln!(
            output,
            "- {} -> {} ({})",
            mapping.source, mapping.target, mapping.scope
        )?;
    }
    writeln!(output, "Applied mappings:")?;
    if report.applied_mappings.is_empty() {
        writeln!(output, "- none")?;
    } else {
        for mapping in &report.applied_mappings {
            writeln!(
                output,
                "- {} -> {} ({})",
                mapping.source, mapping.target, mapping.scope
            )?;
        }
    }
    writeln!(output, "Warnings:")?;
    for warning in &report.warnings {
        writeln!(output, "- {warning}")?;
    }
    writeln!(output, "Migrated spec:")?;
    writeln!(
        output,
        "{}",
        serde_json::to_string_pretty(&report.spec).map_err(io::Error::other)?
    )
}
