fn add_luks_actions(actions: &mut Vec<PlannedAction>, name: &str, luks: &Value) {
    let device = string_field(luks, &["device"]);
    let device_label = device.as_deref().unwrap_or("<device>");
    let mapper_name = string_field(
        luks,
        &["target", "mapperName", "mapper-name", "mapper", "name"],
    )
    .unwrap_or_else(|| name.to_string());
    let operation = luks
        .get("operation")
        .or_else(|| luks.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = luks
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let context = ActionContext {
        collection: Some("luks.devices".to_string()),
        name: Some(name.to_string()),
        target: Some(mapper_name.clone()),
        device: device.clone(),
        property_assignments: property_assignments(luks),
        ..ActionContext::default()
    };
    let has_properties = luks
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| !properties.is_empty());

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:grow"),
            description: format!("resize LUKS mapping {mapper_name} on {device_label}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context,
            advice: Some(Advice {
                summary: "LUKS resize requires backing-device growth and mapper coordination"
                    .to_string(),
                alternatives: vec![
                    "grow the partition, LUN, or volume before resizing the LUKS mapper"
                        .to_string(),
                    "verify the mapping is open and dependent layers are paused or coordinated"
                        .to_string(),
                    "resize filesystems only after cryptsetup resize reports the new size"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy | Operation::Close) => {
            actions.push(luks_close_action(
                name,
                &mapper_name,
                device_label,
                operation.expect("operation already matched"),
                context,
            ));
        }
        Some(Operation::Open) => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Open,
                context,
            ));
        }
        Some(Operation::Create) if preserve_data => {
            actions.push(luks_open_action(
                name,
                &mapper_name,
                device_label,
                Operation::Create,
                context,
            ));
        }
        Some(Operation::Create | Operation::Format) => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "create or replace LUKS container",
        )),
        _ if !preserve_data => actions.push(luks_format_action(
            name,
            device.clone(),
            &mapper_name,
            "preserveData=false permits replacing the LUKS container",
        )),
        _ if !has_properties => actions.push(PlannedAction {
            id: format!("luks.devices:{name}:inspect"),
            description: format!("inspect LUKS declaration {mapper_name} on {device_label}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context,
            advice: None,
        }),
        _ => {}
    }

    add_luks_property_actions(actions, name, &mapper_name, device, luks);
}

fn luks_format_action(
    name: &str,
    device: Option<String>,
    mapper_name: &str,
    description: &str,
) -> PlannedAction {
    let device_label = device.as_deref().unwrap_or("<device>");
    PlannedAction {
        id: format!("luks.devices:{name}:format"),
        description: format!("{description} on {device_label}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("luks.devices".to_string()),
            name: Some(name.to_string()),
            target: Some(mapper_name.to_string()),
            device,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "formatting a LUKS container destroys access to existing encrypted data"
                .to_string(),
            alternatives: vec![
                "open and reuse the existing LUKS container when data must be preserved"
                    .to_string(),
                "back up headers with cryptsetup luksHeaderBackup before destructive work"
                    .to_string(),
                "create a new encrypted target and migrate data before switching mounts"
                    .to_string(),
            ],
        }),
    }
}

fn luks_open_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("open existing LUKS container {device_label} as {mapper_name}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "opening a LUKS mapper changes active device topology without formatting"
                .to_string(),
            alternatives: vec![
                "verify the backing device is the intended LUKS container before opening"
                    .to_string(),
                "use preserveData=false or operation=format only when replacing the header"
                    .to_string(),
                "create filesystems or LVM layers only after the mapper appears".to_string(),
            ],
        }),
    }
}

fn luks_close_action(
    name: &str,
    mapper_name: &str,
    device_label: &str,
    operation: Operation,
    context: ActionContext,
) -> PlannedAction {
    PlannedAction {
        id: format!("luks.devices:{name}:{}", operation_id(operation)),
        description: format!("close LUKS mapping {mapper_name} without formatting {device_label}"),
        operation,
        risk: RiskClass::OfflineRequired,
        destructive: false,
        context,
        advice: Some(Advice {
            summary: "closing a LUKS mapper requires dependent layers to be stopped".to_string(),
            alternatives: vec![
                "unmount filesystems and deactivate LVM volumes before closing the mapper"
                    .to_string(),
                "leave the LUKS header and backing device intact for later reopen".to_string(),
                "use preserveData=false only when reformatting is explicitly intended".to_string(),
            ],
        }),
    }
}

fn add_luks_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    mapper_name: &str,
    device: Option<String>,
    luks: &Value,
) {
    let Some(properties) = luks.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_luks_device_property_change(property);
        actions.push(PlannedAction {
            id: format!("luks.devices:{name}:set-property:{property}"),
            description: format!("set LUKS header property {property} on {mapper_name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                collection: Some("luks.devices".to_string()),
                name: Some(name.to_string()),
                target: Some(mapper_name.to_string()),
                device: device.clone(),
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                property_assignments: property_assignments(luks),
                rollback_value: metadata_string_field(
                    luks,
                    &[
                        "rollbackValue",
                        "rollback-value",
                        "rollback_value",
                        "previousValue",
                        "previous-value",
                        "previous_value",
                        "preApplyValue",
                        "pre-apply-value",
                        "pre_apply_value",
                    ],
                ),
                ..ActionContext::default()
            },
            advice,
        });
    }
}

fn classify_luks_device_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_luks_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: format!(
                    "LUKS header property {property} updates encrypted-container identity metadata"
                ),
                alternatives: vec![
                    "prefer updating consumers to stable by-id paths when possible".to_string(),
                    "back up the LUKS header before changing header identity metadata".to_string(),
                    "verify initrd, crypttab, and NixOS LUKS references after identity changes"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("LUKS header property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, luks.label, subsystem, luks.subsystem, uuid, or luks.uuid for supported LUKS identity changes"
                    .to_string(),
                "use luksKeyslots or luksTokens declarations for access-material changes"
                    .to_string(),
                "apply unsupported LUKS header changes manually after reviewing cryptsetup documentation"
                    .to_string(),
            ],
        }),
    )
}

fn is_luks_identity_property(property: &str) -> bool {
    matches!(
        property,
        "label"
            | "luks.label"
            | "cryptsetup.label"
            | "subsystem"
            | "luks.subsystem"
            | "cryptsetup.subsystem"
            | "uuid"
            | "luks.uuid"
            | "cryptsetup.uuid"
    )
}
