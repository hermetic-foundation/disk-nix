type VerificationResult = (Vec<ExecutionCommand>, Vec<String>);
type VerificationDispatcher = fn(&PlannedAction, VerificationContext<'_>) -> Option<VerificationResult>;

#[derive(Clone, Copy)]
struct VerificationContext<'a> {
    collection: Option<&'a str>,
    target: &'a str,
    cache_target: &'a str,
    mountpoint: Option<&'a str>,
    fs_type: Option<&'a str>,
    desired_size: Option<&'a str>,
}

include!("verification_commands/filesystems_lvm.rs");
include!("verification_commands/local_block.rs");
include!("verification_commands/network.rs");
include!("verification_commands/topology.rs");
include!("verification_commands/properties.rs");
include!("verification_commands/lifecycle.rs");

fn verification_for_action(action: &PlannedAction) -> VerificationResult {
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
        .or_else(|| parts.get(1).copied())
        .unwrap_or("<target>");
    let ctx = VerificationContext {
        collection,
        target,
        cache_target: bcache_target_path(action).unwrap_or(target),
        mountpoint: action.context.mountpoint.as_deref(),
        fs_type: action.context.fs_type.as_deref(),
        desired_size: action.context.desired_size.as_deref(),
    };
    const DISPATCHERS: &[VerificationDispatcher] = &[
        filesystem_lvm_verification,
        local_block_verification,
        network_verification,
        topology_verification,
        property_verification,
        lifecycle_verification,
    ];
    for dispatcher in DISPATCHERS {
        if let Some(result) = dispatcher(action, ctx) {
            return result;
        }
    }
    match action.operation {
        Operation::Format
        | Operation::Shrink
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
        | Operation::Repair
        | Operation::Rollback
        | Operation::Destroy => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after high-risk operation",
            )],
            vec!["operator performs explicit high-risk post-change validation".to_string()],
        ),
        Operation::Grow => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after grow operation",
            )],
            vec!["target capacity and consumers match desired state".to_string()],
        ),
        Operation::Check => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after filesystem check",
            )],
            vec!["read-only check completed and no repair action was applied".to_string()],
        ),
        Operation::Scrub => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after scrub operation",
            )],
            vec!["scrub completed or is running with reviewed health status".to_string()],
        ),
        Operation::Trim => (
            vec![command(
                ["disk-nix", "inspect", target, "--json"],
                false,
                "verify target state after trim operation",
            )],
            vec!["filesystem remains mounted and reports consistent usage after trim".to_string()],
        ),
        _ => (
            vec![command(
                ["disk-nix", "topology", "--json"],
                false,
                "re-probe full topology after high-risk operation",
            )],
            vec!["operator performs explicit high-risk post-change validation".to_string()],
        ),
    }
}
