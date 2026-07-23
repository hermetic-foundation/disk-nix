fn target_lun_provider_capabilities(action: &PlannedAction) -> Vec<String> {
    let mut capabilities = vec![
        "target-lun.identity".to_string(),
        "target-lun.inventory".to_string(),
        "target-lun.persistence".to_string(),
        "target-lun.verification".to_string(),
        "target-lun.refusal".to_string(),
    ];

    match action.operation {
        Operation::Create => {
            capabilities.extend([
                "target-lun.create".to_string(),
                "target-lun.capacity.declare".to_string(),
                "target-lun.backing.bind".to_string(),
                "target-lun.mapping.create".to_string(),
            ]);
        }
        Operation::Grow => {
            capabilities.extend([
                "target-lun.grow".to_string(),
                "target-lun.capacity.expand".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::Attach => {
            capabilities.extend([
                "target-lun.mapping.create".to_string(),
                "target-lun.initiator.allow".to_string(),
            ]);
        }
        Operation::Detach => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.initiator.revoke".to_string(),
            ]);
        }
        Operation::Destroy => {
            capabilities.extend([
                "target-lun.mapping.remove".to_string(),
                "target-lun.destroy".to_string(),
                "target-lun.data-loss.guard".to_string(),
            ]);
        }
        Operation::Rescan => {
            capabilities.extend([
                "target-lun.refresh".to_string(),
                "target-lun.consumer-refresh.handoff".to_string(),
            ]);
        }
        Operation::SetProperty => {
            capabilities.extend([
                "target-lun.property.set".to_string(),
                "target-lun.property.validate".to_string(),
            ]);
        }
        _ => {}
    }

    if action.context.target_id.is_some() {
        capabilities.push("target-lun.target-id.declared".to_string());
    }
    if action.context.vendor.is_some() {
        capabilities.push("target-lun.vendor.declared".to_string());
    }
    if action.context.array_id.is_some() {
        capabilities.push("target-lun.array-id.declared".to_string());
    }
    if action.context.storage_pool.is_some() {
        capabilities.push("target-lun.storage-pool.declared".to_string());
    }
    if action.context.volume_id.is_some() {
        capabilities.push("target-lun.volume-id.declared".to_string());
    }
    if action.context.snapshot_id.is_some() {
        capabilities.push("target-lun.snapshot-id.declared".to_string());
    }
    if action.context.clone_source.is_some() {
        capabilities.push("target-lun.clone-source.declared".to_string());
    }
    if action.context.masking_group.is_some() {
        capabilities.push("target-lun.masking-group.declared".to_string());
    }
    if action.context.lun.is_some() {
        capabilities.push("target-lun.lun-id.declared".to_string());
    }
    if action.context.device.is_some() {
        capabilities.push("target-lun.backing.declared".to_string());
    }
    if action.context.portal.is_some() {
        capabilities.push("target-lun.portal.declared".to_string());
    }
    if action.context.client.is_some() || !action.context.devices.is_empty() {
        capabilities.push("target-lun.initiator-scope.declared".to_string());
    }

    capabilities
}
