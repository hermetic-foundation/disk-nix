fn add_snapshot_actions(actions: &mut Vec<PlannedAction>, name: &str, snapshot: &Value) {
    let target = snapshot
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or(name);
    let snapshot_name = string_field(snapshot, &["name", "snapshotName", "snapshot-name"])
        .unwrap_or_else(|| name.to_string());
    let snapshot_path = string_field(snapshot, &["path", "snapshotPath", "snapshot-path"]);
    let hold = string_field(snapshot, &["hold", "holdTag"]);
    let release_hold = string_field(snapshot, &["releaseHold", "release-hold"]);
    let clone_to = string_field(snapshot, &["cloneTo", "cloneTarget", "clone"]);
    let rename_to = string_field(snapshot, &["renameTo", "renameTarget", "newName"]);
    let destroy = snapshot
        .get("destroy")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let rollback = snapshot
        .get("rollback")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recursive_rollback = snapshot
        .get("recursiveRollback")
        .or_else(|| snapshot.get("recursive"))
        .or_else(|| snapshot.get("zfs.rollbackRecursive"))
        .and_then(Value::as_bool);
    let read_only = snapshot
        .get("readOnly")
        .or_else(|| snapshot.get("readonly"))
        .and_then(Value::as_bool);
    let requested_operation = snapshot
        .get("operation")
        .or_else(|| snapshot.get("action"))
        .and_then(Value::as_str)
        .and_then(parse_operation);

    if requested_operation == Some(Operation::Rescan) {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rescan"),
            description: format!("rescan snapshot metadata for {name}"),
            operation: Operation::Rescan,
            risk: RiskClass::Online,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot rescan refreshes recovery-point metadata without mutating data"
                    .to_string(),
                alternatives: vec![
                    "use holds for retention changes instead of recreating snapshots".to_string(),
                    "clone a snapshot for inspection before rollback or destruction".to_string(),
                    "verify source dataset or subvolume relationships after metadata refresh"
                        .to_string(),
                ],
            }),
        });
    }

    if let Some(hold) = hold {
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &hold,
            read_only,
            false,
        ));
    }
    if let Some(release_hold) = release_hold {
        actions.push(snapshot_hold_action(
            name,
            &snapshot_name,
            target,
            &release_hold,
            read_only,
            true,
        ));
    }
    if let Some(clone_to) = clone_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:clone:{clone_to}"),
            description: format!("clone snapshot {snapshot_name} to {clone_to}"),
            operation: Operation::Clone,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(clone_to),
                snapshot_path: snapshot_path.clone(),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot clone creates a writable ZFS dataset or Btrfs subvolume copy"
                    .to_string(),
                alternatives: vec![
                    "inspect the clone before rollback or destructive changes".to_string(),
                    "destroy the clone after migration or validation if it is no longer needed"
                        .to_string(),
                ],
            }),
        });
    }
    if let Some(rename_to) = rename_to {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rename:{rename_to}"),
            description: format!("rename snapshot {snapshot_name} to {rename_to}"),
            operation: Operation::Rename,
            risk: RiskClass::OfflineRequired,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                rename_to: Some(rename_to),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary:
                    "snapshot rename preserves the recovery point while changing its reference"
                        .to_string(),
                alternatives: vec![
                    "hold the snapshot before renaming when retention jobs may race".to_string(),
                    "update replication, rollback, and cleanup references after rename".to_string(),
                ],
            }),
        });
    }

    if destroy {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:destroy"),
            description: format!("destroy snapshot {snapshot_name} for {target}"),
            operation: Operation::Destroy,
            risk: RiskClass::Destructive,
            destructive: true,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "snapshot destruction removes a recovery point".to_string(),
                alternatives: vec![
                    "keep the snapshot until replacement backups are verified".to_string(),
                    "rename or hold the snapshot before pruning".to_string(),
                ],
            }),
        });
    } else if rollback {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:rollback"),
            description: format!("roll back {target} to snapshot {snapshot_name}"),
            operation: Operation::Rollback,
            risk: RiskClass::PotentialDataLoss,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name.clone()),
                target: Some(target.to_string()),
                read_only,
                recursive_rollback,
                ..ActionContext::default()
            },
            advice: Some(Advice {
                summary: "rollback can discard changes newer than the snapshot".to_string(),
                alternatives: vec![
                    "clone the snapshot and inspect data before rollback".to_string(),
                    "take a pre-rollback snapshot of the current state".to_string(),
                ],
            }),
        });
    } else if actions
        .iter()
        .all(|action| !action.id.starts_with(&format!("snapshot:{name}:")))
    {
        actions.push(PlannedAction {
            id: format!("snapshot:{name}:create"),
            description: format!("create snapshot {snapshot_name} for {target}"),
            operation: Operation::Snapshot,
            risk: RiskClass::Reversible,
            destructive: false,
            context: ActionContext {
                collection: Some("snapshots".to_string()),
                name: Some(snapshot_name),
                target: Some(target.to_string()),
                read_only,
                ..ActionContext::default()
            },
            advice: None,
        });
    }
}

fn snapshot_hold_action(
    action_name: &str,
    snapshot_name: &str,
    target: &str,
    tag: &str,
    read_only: Option<bool>,
    release: bool,
) -> PlannedAction {
    let (verb, property) = if release {
        ("release hold on", "zfs.releaseHold")
    } else {
        ("hold", "zfs.hold")
    };
    PlannedAction {
        id: format!(
            "snapshot:{action_name}:{}:{tag}",
            if release { "release-hold" } else { "hold" }
        ),
        description: format!("{verb} snapshot {snapshot_name} for {target} with tag {tag}"),
        operation: Operation::SetProperty,
        risk: RiskClass::Safe,
        destructive: false,
        context: ActionContext {
            collection: Some("snapshots".to_string()),
            name: Some(snapshot_name.to_string()),
            target: Some(target.to_string()),
            property: Some(property.to_string()),
            property_value: Some(tag.to_string()),
            read_only,
            ..ActionContext::default()
        },
        advice: Some(Advice {
            summary: if release {
                "releasing a snapshot hold allows later pruning by the same tag".to_string()
            } else {
                "holding a snapshot prevents accidental ZFS snapshot destruction by tag".to_string()
            },
            alternatives: if release {
                vec![
                    "keep the hold until replacement backups or replication are verified"
                        .to_string(),
                    "list active holds before releasing retention protection".to_string(),
                ]
            } else {
                vec![
                    "use a stable tag name that identifies the retention policy".to_string(),
                    "replicate or back up the snapshot before removing retention holds".to_string(),
                ]
            },
        }),
    }
}
