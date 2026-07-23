type ActionCommandResult = (Vec<ExecutionCommand>, Vec<String>, bool);
type ActionCommandDispatcher = fn(
    &PlannedAction,
    Option<&str>,
    Option<&str>,
    Option<&str>,
) -> Option<ActionCommandResult>;

include!("action_commands/filesystems.rs");
include!("action_commands/local_block.rs");
include!("action_commands/network.rs");
include!("action_commands/advanced_growth.rs");
include!("action_commands/device_changes.rs");
include!("action_commands/create_open.rs");
include!("action_commands/destroy_lifecycle.rs");
include!("action_commands/remove_rollback.rs");

fn commands_for_action(action: &PlannedAction) -> ActionCommandResult {
    let parts: Vec<&str> = action.id.split(':').collect();
    let collection = action
        .context
        .collection
        .as_deref()
        .or_else(|| parts.first().copied());
    let target = action
        .context
        .target
        .as_deref()
        .or(action.context.name.as_deref())
        .or_else(|| parts.get(1).copied());
    let cache_target = bcache_target_path(action);
    const DISPATCHERS: &[ActionCommandDispatcher] = &[
        filesystem_action_commands,
        local_block_action_commands,
        network_action_commands,
        advanced_growth_action_commands,
        device_change_action_commands,
        create_open_action_commands,
        destroy_lifecycle_action_commands,
        remove_rollback_action_commands,
    ];
    for dispatcher in DISPATCHERS {
        if let Some(result) = dispatcher(action, collection, target, cache_target) {
            return result;
        }
    }
    match action.operation {
        Operation::Format
        | Operation::Shrink
        | Operation::Check
        | Operation::Repair
        | Operation::Clone
        | Operation::Promote
        | Operation::Import
        | Operation::Export
        | Operation::Unexport
        | Operation::Attach
        | Operation::Detach
        | Operation::Activate
        | Operation::Deactivate
        | Operation::Assemble
        | Operation::Start
        | Operation::Stop
        | Operation::Login
        | Operation::Logout
        | Operation::Open
        | Operation::Close
        | Operation::Mount
        | Operation::Unmount
        | Operation::Remount
        | Operation::Rename
        | Operation::Rescan
        | Operation::AddKey
        | Operation::RemoveKey
        | Operation::ImportToken
        | Operation::RemoveToken
        | Operation::RemoveDevice
        | Operation::Rollback
        | Operation::Destroy => (
            vec![unimplemented_action_command(action, collection, target)],
            vec!["no domain-specific command plan is generated for this action yet".to_string()],
            true,
        ),
        _ => (
            vec![unimplemented_action_command(action, collection, target)],
            vec!["no domain-specific command plan is generated for this action yet".to_string()],
            true,
        ),
    }
}

fn zfs_pool_command_target<'a>(action: &'a PlannedAction, fallback: Option<&'a str>) -> &'a str {
    action
        .context
        .name
        .as_deref()
        .or(fallback)
        .unwrap_or("<zfs-pool>")
}
