fn parse_size_bytes(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.ends_with('%') {
        return None;
    }

    let number_end = trimmed
        .find(|character: char| !(character.is_ascii_digit() || character == '.'))
        .unwrap_or(trimmed.len());
    let number = trimmed[..number_end].parse::<f64>().ok()?;
    let unit = trimmed[number_end..].trim().to_ascii_lowercase();
    let multiplier = match unit.as_str() {
        "" | "b" => 1_f64,
        "k" | "kb" => 1_000_f64,
        "m" | "mb" => 1_000_000_f64,
        "g" | "gb" => 1_000_000_000_f64,
        "t" | "tb" => 1_000_000_000_000_f64,
        "p" | "pb" => 1_000_000_000_000_000_f64,
        "ki" | "kib" => 1024_f64,
        "mi" | "mib" => 1024_f64.powi(2),
        "gi" | "gib" => 1024_f64.powi(3),
        "ti" | "tib" => 1024_f64.powi(4),
        "pi" | "pib" => 1024_f64.powi(5),
        _ => return None,
    };

    let bytes = number * multiplier;
    bytes.is_finite().then_some(bytes.round() as u64)
}

fn blocked_action(action: &PlannedAction, policy: &ApplyPolicy) -> Option<BlockedAction> {
    let reason = if action.risk == RiskClass::Unsupported {
        Some("unsupported actions cannot be applied")
    } else if requires_backup(action) && policy.require_backup && !policy.backup_verified {
        Some("backup-required actions require backupVerified=true")
    } else if requires_confirmation(action) && policy.require_confirmation && !policy.confirmation {
        Some("confirmation-required actions require confirmation=true")
    } else if requires_confirmation(action)
        && policy.require_confirmation_file.is_some()
        && !policy.confirmation
    {
        Some(
            "confirmation-file policy requires confirmation=true after checking the configured file",
        )
    } else if action.risk == RiskClass::OfflineRequired && !policy.allow_offline {
        Some("offline-required actions require allowOffline=true")
    } else if action.operation == Operation::Format && !policy.allow_format {
        Some("format actions require allowFormat=true")
    } else if action.operation == Operation::Shrink && !policy.allow_shrink {
        Some("shrink actions require allowShrink=true")
    } else if action.risk == RiskClass::PotentialDataLoss && !policy.allow_potential_data_loss {
        Some("potential-data-loss actions require allowPotentialDataLoss=true")
    } else if action.operation == Operation::Grow && !policy.allow_grow {
        Some("grow actions require allowGrow=true")
    } else if matches!(
        action.operation,
        Operation::AddDevice | Operation::ReplaceDevice | Operation::RemoveDevice
    ) && !policy.allow_device_replacement
    {
        Some("device topology changes require allowDeviceReplacement=true")
    } else if action.operation == Operation::Rebalance && !policy.allow_rebalance {
        Some("rebalance actions require allowRebalance=true")
    } else if action.operation == Operation::SetProperty && !policy.allow_property_changes {
        Some("property changes require allowPropertyChanges=true")
    } else if action.operation == Operation::Format && !policy.allow_destructive {
        Some("format actions also require allowDestructive=true")
    } else if action.destructive
        || action.risk == RiskClass::Destructive
        || action.risk == RiskClass::Irreversible
    {
        (!policy.allow_destructive)
            .then_some("destructive or irreversible actions require allowDestructive=true")
    } else {
        None
    }?;

    Some(BlockedAction {
        id: action.id.clone(),
        operation: action.operation,
        risk: action.risk,
        reason: reason.to_string(),
    })
}

fn requires_backup(action: &PlannedAction) -> bool {
    action.destructive
        || matches!(
            action.risk,
            RiskClass::PotentialDataLoss | RiskClass::Destructive | RiskClass::Irreversible
        )
}

fn requires_confirmation(action: &PlannedAction) -> bool {
    requires_backup(action)
        || matches!(
            action.risk,
            RiskClass::OfflineRequired | RiskClass::Unsupported
        )
}
