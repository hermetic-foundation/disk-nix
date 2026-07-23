fn classify_network_storage_operation(
    collection: &str,
    operation: Operation,
    object: &Value,
) -> Option<OperationClassification> {
    let _ = object;
    Some(match operation {
        Operation::Create | Operation::Export if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export creation publishes an existing path to selected clients"
                    .to_string(),
                alternatives: vec![
                    "export read-only first when client behavior is unknown".to_string(),
                    "restrict clients and options before enabling write access".to_string(),
                    "verify the source path and ownership before reloading exports".to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "exports" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS export rescan refreshes exported path and client visibility"
                    .to_string(),
                alternatives: vec![
                    "use option property updates only when client access semantics must change"
                        .to_string(),
                    "verify active clients before unexporting or tightening access".to_string(),
                    "persist long-lived exports through NixOS services.nfs.server.exports"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Login if collection == "iscsiSessions" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "iSCSI session login discovers target portals and attaches remote LUNs"
                    .to_string(),
                alternatives: vec![
                    "verify the portal and target IQN before logging in".to_string(),
                    "prefer stable multipath and by-id consumers before resizing filesystems"
                        .to_string(),
                    "keep NixOS open-iscsi session declarations aligned with imperative login"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Mount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting an NFS client path changes host namespace state".to_string(),
                alternatives: vec![
                    "use x-systemd.automount or nofail when the server may be unavailable"
                        .to_string(),
                    "verify DNS, routing, firewall, and export permissions before mounting"
                        .to_string(),
                    "prefer declarative NixOS fileSystems for steady-state client mounts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Mount if collection == "filesystems" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "mounting a filesystem changes local namespace state without formatting storage"
                    .to_string(),
                alternatives: vec![
                    "verify the source device, filesystem type, and mountpoint before mounting"
                        .to_string(),
                    "prefer x-systemd.automount, nofail, or service ordering when dependencies may be unavailable"
                        .to_string(),
                    "persist long-lived mounts through the matching NixOS fileSystems entry"
                        .to_string(),
                ],
            }),
        ),
        Operation::Remount if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "remounting an NFS client path updates local mount options without deleting remote data"
                    .to_string(),
                alternatives: vec![
                    "prefer remounting with reviewed options before unmounting a busy path"
                        .to_string(),
                    "use NixOS fileSystems for the steady-state mount options".to_string(),
                    "verify active services tolerate option changes such as ro, rw, or timeouts"
                        .to_string(),
                ],
            }),
        ),
        Operation::Rescan if collection == "nfs.mounts" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "NFS mount rescan refreshes local mount source, options, and client stats"
                    .to_string(),
                alternatives: vec![
                    "use remount only when local mount options must change".to_string(),
                    "verify server reachability before unmounting busy client paths".to_string(),
                    "persist long-lived mounts through NixOS fileSystems".to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Attach if collection == "luns" => (
            RiskClass::Online,
            false,
            Some(Advice {
                summary: "LUN host attach makes an existing target-side LUN visible to this host"
                    .to_string(),
                alternatives: vec![
                    "create or grow the target-side LUN before host attach".to_string(),
                    "declare stable by-path devices so apply can verify every expected path"
                        .to_string(),
                    "keep multipath and filesystem consumers disabled until paths are verified"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create | Operation::Attach if collection == "targetLuns" => (
            RiskClass::OfflineRequired,
            false,
            Some(Advice {
                summary:
                    "target-side LUN provisioning allocates or maps storage on an external target"
                        .to_string(),
                alternatives: vec![
                    "use an array-specific provider or storage runbook before host-side attach"
                        .to_string(),
                    "record the backing object, initiator mapping, LUN number, and expected size before rescanning hosts"
                        .to_string(),
                    "prefer mapping an existing reviewed LUN when preserving data is required"
                        .to_string(),
                ],
            }),
        ),
        Operation::Create if collection == "nvmeNamespaces" => (
            RiskClass::Destructive,
            true,
            Some(Advice {
                summary: "NVMe namespace creation allocates controller-managed namespace capacity"
                    .to_string(),
                alternatives: vec![
                    "attach an existing namespace when preserving data is required".to_string(),
                    "verify controller namespace inventory before create-ns".to_string(),
                    "declare namespaceId and controllers before attaching the created namespace"
                        .to_string(),
                ],
            }),
        ),
        _ => return None,
    })
}
