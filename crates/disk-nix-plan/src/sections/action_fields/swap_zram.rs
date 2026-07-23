fn add_swap_actions(actions: &mut Vec<PlannedAction>, name: &str, swap: &Value) {
    let device =
        string_field(swap, &["target", "path", "device"]).unwrap_or_else(|| name.to_string());
    let operation = swap
        .get("operation")
        .or_else(|| swap.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let preserve_data = swap
        .get("preserveData")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let desired_size = desired_size(swap);
    let context = ActionContext {
        collection: Some("swaps".to_string()),
        name: Some(name.to_string()),
        target: Some(device.clone()),
        device: Some(device.clone()),
        desired_size: desired_size.clone(),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Grow) => actions.push(PlannedAction {
            id: format!("swaps:{name}:grow"),
            description: format!("grow swap backing storage for {device}"),
            operation: Operation::Grow,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap growth requires disabling active swap before resizing backing storage"
                        .to_string(),
                alternatives: vec![
                    "add a second swap device before resizing this one".to_string(),
                    "disable swap, resize backing storage, recreate the signature, and re-enable"
                        .to_string(),
                    "verify memory pressure and hibernation dependencies before disabling swap"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: format!("swaps:{name}:rescan"),
            description: format!("refresh swap inventory for {device}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap rescan refreshes signature, activation, and graph inventory"
                    .to_string(),
                alternatives: vec![
                    "use grow when backing swap capacity must change".to_string(),
                    "use format only when replacing the swap signature is intended".to_string(),
                    "verify resume and hibernation references before changing swap identity"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Deactivate | Operation::Stop) => actions.push(PlannedAction {
            id: format!("swaps:{name}:deactivate"),
            description: format!("disable active swap on {device}"),
            operation: Operation::Deactivate,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "swap deactivation runs swapoff without removing the swap signature"
                    .to_string(),
                alternatives: vec![
                    "add replacement swap capacity before disabling active swap".to_string(),
                    "use destroy only when the swap signature should be removed".to_string(),
                    "verify resume and hibernation references before disabling swap".to_string(),
                ],
            }),
        }),
        Some(Operation::Destroy) => actions.push(PlannedAction {
            id: format!("swaps:{name}:destroy"),
            description: format!("disable swap and remove swap signature from {device}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: context.clone(),
            advice: Some(Advice {
                summary:
                    "swap destruction disables active swap and removes swap signature metadata"
                        .to_string(),
                alternatives: vec![
                    "use operation = \"deactivate\" to run swapoff without removing the signature"
                        .to_string(),
                    "remove or update NixOS swapDevices before deleting the swap signature"
                        .to_string(),
                    "verify resume and hibernation references before wiping swap metadata"
                        .to_string(),
                ],
            }),
        }),
        Some(Operation::Create | Operation::Format) => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "create or refresh swap signature",
        )),
        _ if !preserve_data => actions.push(swap_format_action(
            name,
            &device,
            desired_size,
            "preserveData=false permits recreating the swap signature",
        )),
        _ => actions.push(PlannedAction {
            id: format!("swaps:{name}:inspect"),
            description: format!("inspect swap declaration for {device}"),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_swap_property_actions(actions, name, swap, &context);
}

fn add_swap_property_actions(
    actions: &mut Vec<PlannedAction>,
    name: &str,
    swap: &Value,
    context: &ActionContext,
) {
    let Some(properties) = swap.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        let (risk, advice) = classify_swap_property_change(property);
        actions.push(PlannedAction {
            id: format!("swaps:{name}:set-property:{property}"),
            description: format!("set swap property {property} on {name}"),
            operation: Operation::SetProperty,
            risk,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                rollback_value: metadata_string_field(
                    swap,
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
                ..context.clone()
            },
            advice,
        });
    }
}

fn classify_swap_property_change(property: &str) -> (RiskClass, Option<Advice>) {
    if is_swap_identity_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "swap label and UUID updates mutate swap signature identity".to_string(),
                alternatives: vec![
                    "prefer updating NixOS swapDevices references to the current identity when possible"
                        .to_string(),
                    "disable active swap and verify hibernation/resume references before changing identity"
                        .to_string(),
                    "use a stable device path instead of changing swap UUID when consumers allow it"
                        .to_string(),
                ],
            }),
        );
    }
    if is_swap_priority_property(property) {
        return (
            RiskClass::OfflineRequired,
            Some(Advice {
                summary: "swap priority updates reactivate the reviewed swap target".to_string(),
                alternatives: vec![
                    "prefer changing NixOS swapDevices priority for steady-state configuration"
                        .to_string(),
                    "review memory pressure and hibernation/resume state before swapoff".to_string(),
                    "use a temporary additional swap device before changing priority on busy systems"
                        .to_string(),
                ],
            }),
        );
    }

    (
        RiskClass::Unsupported,
        Some(Advice {
            summary: format!("swap property {property} is not mapped to a safe command"),
            alternatives: vec![
                "use label, swap.label, uuid, swap.uuid, priority, or swap.priority for supported swap changes"
                    .to_string(),
                "recreate the swap signature with preserveData=false only when overwriting metadata is intended"
                    .to_string(),
                "apply unsupported swap changes manually after reviewing util-linux swap tools"
                    .to_string(),
            ],
        }),
    )
}

fn is_swap_identity_property(property: &str) -> bool {
    matches!(property, "label" | "swap.label" | "uuid" | "swap.uuid")
}

fn is_swap_priority_property(property: &str) -> bool {
    matches!(property, "priority" | "swap.priority")
}

fn add_zram_actions(actions: &mut Vec<PlannedAction>, zram: &Map<String, Value>) {
    let operation = zram
        .get("operation")
        .or_else(|| zram.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);
    let context = ActionContext {
        collection: Some("zram".to_string()),
        name: Some("zram".to_string()),
        target: Some("zram".to_string()),
        ..ActionContext::default()
    };

    match operation {
        Some(Operation::Rescan) => actions.push(PlannedAction {
            id: "zram:rescan".to_string(),
            description: "refresh zram compressed swap inventory".to_string(),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: context.clone(),
            advice: Some(Advice {
                summary: "zram rescan refreshes generated compressed swap state".to_string(),
                alternatives: vec![
                    "review zramctl output before changing generated zramSwap settings".to_string(),
                    "coordinate swapoff and setup when active zram devices must be recreated"
                        .to_string(),
                ],
            }),
        }),
        _ => actions.push(PlannedAction {
            id: "zram:inspect".to_string(),
            description: "inspect zram compressed swap declaration".to_string(),
            operation: Operation::SetProperty,
            risk: RiskClass::Safe,
            destructive: false,
            context: context.clone(),
            advice: None,
        }),
    }

    add_zram_property_actions(actions, zram, &context);
}

fn add_zram_property_actions(
    actions: &mut Vec<PlannedAction>,
    zram: &Map<String, Value>,
    context: &ActionContext,
) {
    let Some(properties) = zram.get("properties").and_then(Value::as_object) else {
        return;
    };

    for (property, value) in properties {
        actions.push(PlannedAction {
            id: format!("zram:set-property:{property}"),
            description: format!("set zram property {property}"),
            operation: Operation::SetProperty,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                property: Some(property.to_string()),
                property_value: Some(property_value(value)),
                ..context.clone()
            },
            advice: Some(Advice {
                summary: format!("zram property {property} requires generator reconciliation"),
                alternatives: vec![
                    "use services.disk-nix.zram options to derive NixOS zramSwap".to_string(),
                    "run a zram rescan before recreating active compressed swap devices".to_string(),
                    "coordinate swapoff before changing live zram algorithm, priority, size, or writeback device"
                        .to_string(),
                ],
            }),
        });
    }
}

fn swap_format_action(
    name: &str,
    device: &str,
    desired_size: Option<String>,
    description: &str,
) -> PlannedAction {
    PlannedAction {
        id: format!("swaps:{name}:format"),
        description: format!("{description} on {device}"),
        operation: Operation::Format,
        risk: RiskClass::Destructive,
        destructive: true,
        context: ActionContext {
            collection: Some("swaps".to_string()),
            name: Some(name.to_string()),
            target: Some(device.to_string()),
            device: Some(device.to_string()),
            desired_size,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: "creating a swap signature overwrites existing metadata on the target"
                .to_string(),
            alternatives: vec![
                "use an additional swap file or device instead of replacing this target"
                    .to_string(),
                "verify the target contains no filesystem or encrypted data before mkswap"
                    .to_string(),
                "set preserveData=true for inspection-only planning".to_string(),
            ],
        }),
    }
}
