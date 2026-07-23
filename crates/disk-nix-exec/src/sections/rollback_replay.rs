#[must_use]
pub fn replay_proven_safe_rollback_recipe(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
) -> RollbackExecutionReport {
    let fresh_topology_probe_id = fresh_topology_probe_id.into();
    let topology_evidence =
        materialize_rollback_topology_evidence(failed_report, &fresh_topology_probe_id);
    replay_proven_safe_rollback_recipe_with_topology_evidence(
        failed_report,
        recipe_index,
        original_receipt_id,
        fresh_topology_probe_id,
        topology_evidence,
    )
}

#[must_use]
pub fn materialize_rollback_topology_evidence(
    failed_report: &ExecutionReport,
    fresh_topology_probe_id: &str,
) -> BTreeMap<String, String> {
    let mut topology_evidence = BTreeMap::from([
        (
            "expected".to_string(),
            topology_evidence_id("expected", &failed_report.apply),
        ),
        (
            "preApply".to_string(),
            topology_evidence_id(
                "pre-apply",
                &(
                    &failed_report.topology_comparison,
                    &failed_report.command_plan,
                    &failed_report.verification_plan,
                ),
            ),
        ),
        (
            "failedApply".to_string(),
            topology_evidence_id(
                "failed-apply",
                &(
                    failed_report.status,
                    &failed_report.partial_execution_recovery,
                    &failed_report.execution_results,
                ),
            ),
        ),
    ]);
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("current".to_string(), fresh_topology_probe_id.to_string());
    }
    topology_evidence
}

#[must_use]
pub fn materialize_rollback_topology_payloads(
    failed_report: &ExecutionReport,
    current_topology_payload: serde_json::Value,
) -> BTreeMap<String, serde_json::Value> {
    BTreeMap::from([
        (
            "expected".to_string(),
            serde_json::to_value(&failed_report.apply).unwrap_or(serde_json::Value::Null),
        ),
        (
            "preApply".to_string(),
            serde_json::to_value((
                &failed_report.topology_comparison,
                &failed_report.command_plan,
                &failed_report.verification_plan,
            ))
            .unwrap_or(serde_json::Value::Null),
        ),
        (
            "failedApply".to_string(),
            serde_json::to_value((
                failed_report.status,
                &failed_report.partial_execution_recovery,
                &failed_report.execution_results,
            ))
            .unwrap_or(serde_json::Value::Null),
        ),
        ("current".to_string(), current_topology_payload),
    ])
}

#[must_use]
pub fn replay_proven_safe_rollback_recipe_with_topology_evidence(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
    topology_evidence: BTreeMap<String, String>,
) -> RollbackExecutionReport {
    let mut runner = run_command;
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id: original_receipt_id.into(),
            fresh_topology_probe_id: fresh_topology_probe_id.into(),
            topology_evidence,
            topology_payloads: BTreeMap::new(),
        },
        &mut runner,
        command_exists,
    )
}

#[must_use]
pub fn replay_proven_safe_rollback_recipe_with_topology_payloads(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: impl Into<String>,
    fresh_topology_probe_id: impl Into<String>,
    topology_evidence: BTreeMap<String, String>,
    topology_payloads: BTreeMap<String, serde_json::Value>,
) -> RollbackExecutionReport {
    let mut runner = run_command;
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id: original_receipt_id.into(),
            fresh_topology_probe_id: fresh_topology_probe_id.into(),
            topology_evidence,
            topology_payloads,
        },
        &mut runner,
        command_exists,
    )
}

#[cfg(test)]
fn replay_proven_safe_rollback_recipe_with_runner(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    original_receipt_id: String,
    fresh_topology_probe_id: String,
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
) -> RollbackExecutionReport {
    let mut topology_evidence = current_topology_evidence(&fresh_topology_probe_id);
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("expected".to_string(), "topology:expected-123".to_string());
        topology_evidence.insert("preApply".to_string(), "topology:pre-apply-123".to_string());
        topology_evidence.insert(
            "failedApply".to_string(),
            "topology:failed-apply-123".to_string(),
        );
    }
    replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
        failed_report,
        recipe_index,
        RollbackReplayBindings {
            original_receipt_id,
            fresh_topology_probe_id,
            topology_evidence,
            topology_payloads: BTreeMap::new(),
        },
        runner,
        |_| true,
    )
}

fn replay_proven_safe_rollback_recipe_with_runner_and_tool_checker(
    failed_report: &ExecutionReport,
    recipe_index: usize,
    bindings: RollbackReplayBindings,
    runner: &mut impl FnMut(&[String]) -> CommandRunResult,
    tool_exists: impl Fn(&str) -> bool,
) -> RollbackExecutionReport {
    let Some(recipe) = failed_report.rollback_recipes.get(recipe_index) else {
        return refused_rollback_report(
            0,
            "",
            &[],
            bindings,
            vec!["rollback recipe index does not exist".to_string()],
        );
    };

    let mut refusal_reasons = proven_safe_rollback_refusal_reasons(
        failed_report,
        recipe,
        &bindings.original_receipt_id,
        &bindings.fresh_topology_probe_id,
        &bindings.topology_evidence,
        tool_exists,
    );
    if !refusal_reasons.is_empty() {
        refusal_reasons.extend(recipe.refusal_reasons.iter().cloned());
        return refused_rollback_report(
            recipe.recipe_version,
            &recipe.source_action_id,
            &recipe.failed_command,
            bindings,
            refusal_reasons,
        );
    }

    let mut validation_results = Vec::new();
    for command in &recipe.read_only_validation.commands {
        let result = run_planned_command(
            ExecutionPhase::Verification,
            &recipe.source_action_id,
            &command.argv,
            runner,
        );
        let success = result.success;
        validation_results.push(result);
        if !success {
            return RollbackExecutionReport {
                status: RollbackExecutionStatus::Failed,
                recipe_version: recipe.recipe_version,
                source_action_id: recipe.source_action_id.clone(),
                receipt_binding: rollback_receipt_binding(recipe, bindings.clone()),
                validation_results,
                rollback_results: Vec::new(),
                messages: vec![
                    "rollback validation failed; reversible mutation steps were not executed"
                        .to_string(),
                ],
                refusal_reasons: Vec::new(),
            };
        }
    }

    let mut rollback_results = Vec::new();
    for command in &recipe.reversible_mutations.commands {
        let result = run_planned_command(
            ExecutionPhase::Command,
            &recipe.source_action_id,
            &command.argv,
            runner,
        );
        let success = result.success;
        rollback_results.push(result);
        if !success {
            return RollbackExecutionReport {
                status: RollbackExecutionStatus::Failed,
                recipe_version: recipe.recipe_version,
                source_action_id: recipe.source_action_id.clone(),
                receipt_binding: rollback_receipt_binding(
                    recipe,
                    bindings.clone(),
                ),
                validation_results,
                rollback_results,
                messages: vec![
                    "proven-safe rollback mutation failed; capture a fresh topology probe before retrying or handoff".to_string(),
                ],
                refusal_reasons: Vec::new(),
            };
        }
    }

    RollbackExecutionReport {
        status: RollbackExecutionStatus::Succeeded,
        recipe_version: recipe.recipe_version,
        source_action_id: recipe.source_action_id.clone(),
        receipt_binding: rollback_receipt_binding(recipe, bindings),
        validation_results,
        rollback_results,
        messages: vec![
            "proven-safe rollback validation and reversible mutation steps completed".to_string(),
            "capture and compare a fresh topology probe after rollback before resuming apply"
                .to_string(),
        ],
        refusal_reasons: Vec::new(),
    }
}

fn proven_safe_rollback_refusal_reasons(
    failed_report: &ExecutionReport,
    recipe: &RollbackRecipe,
    original_receipt_id: &str,
    fresh_topology_probe_id: &str,
    topology_evidence: &BTreeMap<String, String>,
    tool_exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut reasons = Vec::new();

    if failed_report.status != ExecutionStatus::Failed {
        reasons.push("automatic rollback replay requires a failed apply report".to_string());
    }
    if recipe.status != RollbackRecipeStatus::ProvenSafe {
        reasons.push("rollback recipe is not marked proven-safe".to_string());
    }
    if recipe.receipt_binding_required && original_receipt_id.trim().is_empty() {
        reasons.push("original apply receipt binding is required".to_string());
    }
    if recipe.fresh_topology_probe_required && fresh_topology_probe_id.trim().is_empty() {
        reasons.push("fresh post-failure topology probe binding is required".to_string());
    }
    let missing_topology_evidence = missing_required_topology_evidence(recipe, topology_evidence);
    if !missing_topology_evidence.is_empty() {
        reasons.push(format!(
            "automatic rollback replay refuses missing topology evidence binding(s): {}",
            missing_topology_evidence.join(", ")
        ));
    }
    reasons.extend(rollback_topology_comparison_refusal_reasons(failed_report));
    if !recipe.destructive_mutations.commands.is_empty() {
        reasons.push("automatic rollback replay refuses destructive mutation steps".to_string());
    }
    if !recipe.operator_only_handoff.commands.is_empty() {
        reasons.push("automatic rollback replay refuses operator-only handoff steps".to_string());
    }
    if recipe.reversible_mutations.commands.is_empty() {
        reasons.push("rollback recipe has no proven-safe reversible mutation steps".to_string());
    }
    let missing_tools = missing_rollback_replay_tools(recipe, tool_exists);
    if !missing_tools.is_empty() {
        reasons.push(format!(
            "automatic rollback replay refuses missing required tool(s): {}",
            missing_tools.join(", ")
        ));
    }

    for command in &recipe.read_only_validation.commands {
        if command.mutates {
            reasons.push(format!(
                "read-only validation command mutates state: {}",
                command.argv.join(" ")
            ));
        }
        if command.readiness != CommandReadiness::Ready {
            reasons.push(format!(
                "read-only validation command is not ready: {}",
                command.argv.join(" ")
            ));
        }
    }
    for command in &recipe.reversible_mutations.commands {
        if !command.mutates {
            reasons.push(format!(
                "reversible rollback command is not marked mutating: {}",
                command.argv.join(" ")
            ));
        }
        if command.readiness != CommandReadiness::Ready {
            reasons.push(format!(
                "reversible rollback command is not ready: {}",
                command.argv.join(" ")
            ));
        }
        if let Some(reason) = rollback_command_data_loss_risk_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_live_use_blocker_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_identity_blocker_reason(command) {
            reasons.push(reason);
        }
        if let Some(reason) = rollback_command_idempotency_blocker_reason(command) {
            reasons.push(reason);
        }
    }

    reasons
}

fn rollback_command_data_loss_risk_reason(command: &ExecutionCommand) -> Option<String> {
    let risky_arg_tokens = [
        "destroy",
        "delete",
        "detach",
        "discard",
        "flush",
        "format",
        "kill-slot",
        "remove",
        "rollback",
        "shrink",
        "wipe",
    ];
    if command.argv.iter().any(|part| {
        let part = part.to_ascii_lowercase();
        risky_arg_tokens
            .iter()
            .any(|token| part == *token || part.starts_with(&format!("{token}=")))
    }) {
        return Some(format!(
            "automatic rollback replay refuses plausible data-loss command: {}",
            command.argv.join(" ")
        ));
    }

    let risky_phrases = [
        "data loss",
        "data-loss",
        "destructive",
        "discard data",
        "discard newer data",
        "format",
        "potential data loss",
        "potential-data-loss",
        "shrink",
        "wipe",
    ];
    let mut risk_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if risk_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        risky_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses plausible data-loss command metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_live_use_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "active consumer",
        "active consumers",
        "active session",
        "active sessions",
        "exported lun",
        "exported luns",
        "holder",
        "holders",
        "live mapping",
        "live mappings",
        "mounted filesystem",
        "mounted filesystems",
        "mounted",
        "open encrypted mapping",
        "open encrypted mappings",
        "open mapping",
        "open mappings",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses live-use blocker metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_identity_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "ambiguous rollback point",
        "ambiguous rollback target",
        "ambiguous target",
        "rollback point missing",
        "rollback point stale",
        "stale identity",
        "stale identity data",
        "stale rollback point",
        "stale target identity",
        "unbound rollback point",
        "unbound rollback target",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses ambiguous or stale identity metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

fn rollback_command_idempotency_blocker_reason(command: &ExecutionCommand) -> Option<String> {
    let blocker_phrases = [
        "already rolled back",
        "already-rolled-back",
        "external modification",
        "external modifications",
        "externally modified",
        "partially rolled back",
        "partially-rolled-back",
        "rollback already applied",
        "rollback partially applied",
        "rollback state diverged",
        "topology externally modified",
    ];
    let mut metadata_fields = command
        .provider_capabilities
        .iter()
        .chain(command.unresolved_inputs.iter())
        .chain(std::iter::once(&command.note));
    if metadata_fields.any(|field| {
        let field = field.to_ascii_lowercase();
        blocker_phrases.iter().any(|phrase| field.contains(phrase))
    }) {
        return Some(format!(
            "automatic rollback replay refuses idempotency blocker metadata: {}",
            command.argv.join(" ")
        ));
    }

    None
}

#[cfg(test)]
fn current_topology_evidence(fresh_topology_probe_id: &str) -> BTreeMap<String, String> {
    let mut topology_evidence = BTreeMap::new();
    if !fresh_topology_probe_id.trim().is_empty() {
        topology_evidence.insert("current".to_string(), fresh_topology_probe_id.to_string());
    }
    topology_evidence
}

fn topology_evidence_id(label: &str, value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_else(|_| label.as_bytes().to_vec());
    format!("topology:{label}:{:016x}", fnv1a64(&bytes))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn missing_required_topology_evidence(
    recipe: &RollbackRecipe,
    topology_evidence: &BTreeMap<String, String>,
) -> Vec<String> {
    recipe
        .required_topology_evidence
        .iter()
        .filter_map(|label| {
            let present = topology_evidence
                .get(label)
                .is_some_and(|evidence_id| !evidence_id.trim().is_empty());
            (!present).then(|| label.clone())
        })
        .collect()
}

fn rollback_topology_comparison_refusal_reasons(failed_report: &ExecutionReport) -> Vec<String> {
    let Some(comparison) = failed_report.topology_comparison.as_ref() else {
        return Vec::new();
    };
    let summary = &comparison.summary;
    let mut divergences = Vec::new();
    if summary.missing_count > 0 {
        divergences.push(format!("{} missing target(s)", summary.missing_count));
    }
    if summary.size_diagnostic_count > 0 {
        divergences.push(format!(
            "{} size diagnostic(s)",
            summary.size_diagnostic_count
        ));
    }
    if summary.type_conflict_count > 0 {
        divergences.push(format!(
            "{} type conflict diagnostic(s)",
            summary.type_conflict_count
        ));
    }
    if summary.graph_dependency_conflict_count > 0 {
        divergences.push(format!(
            "{} graph dependency conflict(s)",
            summary.graph_dependency_conflict_count
        ));
    }
    if summary.partially_suppressed_group_count > 0 {
        divergences.push(format!(
            "{} partially suppressed reconciliation group(s)",
            summary.partially_suppressed_group_count
        ));
    }
    divergences.extend(rollback_topology_diagnostic_refusal_reasons(comparison));

    if divergences.is_empty() {
        Vec::new()
    } else {
        vec![format!(
            "automatic rollback replay refuses divergent topology comparison: {}",
            divergences.join(", ")
        )]
    }
}

fn rollback_topology_diagnostic_refusal_reasons(comparison: &TopologyComparison) -> Vec<String> {
    let mut live_use = BTreeSet::new();
    let mut stale_identity = BTreeSet::new();
    let mut idempotency = BTreeSet::new();
    let mut data_loss = BTreeSet::new();

    for diagnostic in &comparison.diagnostics {
        if rollback_topology_diagnostic_is_live_use_blocker(diagnostic.kind) {
            live_use.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_stale_identity_blocker(diagnostic.kind) {
            stale_identity.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_idempotency_blocker(diagnostic.kind) {
            idempotency.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
        if rollback_topology_diagnostic_is_data_loss_risk(diagnostic.kind) {
            data_loss.insert(rollback_topology_diagnostic_label(
                &diagnostic.action_id,
                diagnostic.kind,
            ));
        }
    }

    let mut reasons = Vec::new();
    if !live_use.is_empty() {
        reasons.push(format!(
            "topology diagnostic live-use blocker(s): {}",
            live_use.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !stale_identity.is_empty() {
        reasons.push(format!(
            "topology diagnostic stale identity or ambiguous rollback point(s): {}",
            stale_identity.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !idempotency.is_empty() {
        reasons.push(format!(
            "topology diagnostic rollback idempotency blocker(s): {}",
            idempotency.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if !data_loss.is_empty() {
        reasons.push(format!(
            "topology diagnostic plausible data-loss path(s): {}",
            data_loss.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    reasons
}

fn rollback_topology_diagnostic_label(action_id: &str, kind: TopologyDiagnosticKind) -> String {
    format!("{action_id}:{kind:?}")
}

fn rollback_topology_diagnostic_is_live_use_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::MountRequired
            | TopologyDiagnosticKind::MountOptionsDiffer
            | TopologyDiagnosticKind::UnmountRequired
            | TopologyDiagnosticKind::NfsExportDiffers
            | TopologyDiagnosticKind::NfsExportRequired
            | TopologyDiagnosticKind::NfsUnexportRequired
            | TopologyDiagnosticKind::IscsiLoginRequired
            | TopologyDiagnosticKind::IscsiLogoutRequired
            | TopologyDiagnosticKind::LunAttachRequired
            | TopologyDiagnosticKind::LunDetachRequired
            | TopologyDiagnosticKind::NvmeNamespaceAttachRequired
            | TopologyDiagnosticKind::NvmeNamespaceDetachRequired
            | TopologyDiagnosticKind::LvmActivateRequired
            | TopologyDiagnosticKind::LvmDeactivateRequired
            | TopologyDiagnosticKind::LvmVgExportRequired
            | TopologyDiagnosticKind::LvmVgImportRequired
            | TopologyDiagnosticKind::LuksCloseRequired
            | TopologyDiagnosticKind::LuksOpenRequired
            | TopologyDiagnosticKind::DmMapDestroyRequired
            | TopologyDiagnosticKind::DmMapRenameRequired
            | TopologyDiagnosticKind::MultipathDestroyRequired
            | TopologyDiagnosticKind::MultipathPathAddRequired
            | TopologyDiagnosticKind::MultipathPathRemoveRequired
            | TopologyDiagnosticKind::SwapDeactivateRequired
            | TopologyDiagnosticKind::LoopDetachRequired
            | TopologyDiagnosticKind::MdStopRequired
            | TopologyDiagnosticKind::VdoStartRequired
            | TopologyDiagnosticKind::VdoStopRequired
    )
}

fn rollback_topology_diagnostic_is_stale_identity_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::Missing
            | TopologyDiagnosticKind::MountSourceConflict
            | TopologyDiagnosticKind::LoopCreateConflict
            | TopologyDiagnosticKind::LuksFormatTargetPresent
            | TopologyDiagnosticKind::SwapFormatTargetPresent
            | TopologyDiagnosticKind::VdoCreateTargetPresent
            | TopologyDiagnosticKind::SnapshotCloneSourceMissing
            | TopologyDiagnosticKind::SnapshotRenameSourceMissing
            | TopologyDiagnosticKind::SnapshotRollbackPointMissing
    )
}

fn rollback_topology_diagnostic_is_idempotency_blocker(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::Matched
            | TopologyDiagnosticKind::SizeAlreadySatisfied
            | TopologyDiagnosticKind::FilesystemFormatAlreadySatisfied
            | TopologyDiagnosticKind::DiskCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsSubvolumeCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsSubvolumeDestroyAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsQgroupCreateAlreadySatisfied
            | TopologyDiagnosticKind::BtrfsQgroupDestroyAlreadySatisfied
            | TopologyDiagnosticKind::BcacheDetachAlreadySatisfied
            | TopologyDiagnosticKind::BackingFileCreateAlreadySatisfied
            | TopologyDiagnosticKind::PartitionCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmPvCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVolumeCreateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgCreateAlreadySatisfied
            | TopologyDiagnosticKind::IscsiLoginAlreadySatisfied
            | TopologyDiagnosticKind::IscsiLogoutAlreadySatisfied
            | TopologyDiagnosticKind::LunAttachAlreadySatisfied
            | TopologyDiagnosticKind::LunDetachAlreadySatisfied
            | TopologyDiagnosticKind::DmMapDestroyAlreadySatisfied
            | TopologyDiagnosticKind::DmMapRenameAlreadySatisfied
            | TopologyDiagnosticKind::NvmeNamespaceAttachAlreadySatisfied
            | TopologyDiagnosticKind::NvmeNamespaceDetachAlreadySatisfied
            | TopologyDiagnosticKind::LvmActivateAlreadySatisfied
            | TopologyDiagnosticKind::LvmDeactivateAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgExportAlreadySatisfied
            | TopologyDiagnosticKind::LvmVgImportAlreadySatisfied
            | TopologyDiagnosticKind::LvmRenameAlreadySatisfied
            | TopologyDiagnosticKind::LvmCacheDetachAlreadySatisfied
            | TopologyDiagnosticKind::LuksCloseAlreadySatisfied
            | TopologyDiagnosticKind::LuksOpenAlreadySatisfied
            | TopologyDiagnosticKind::LuksKeyslotRemoveAlreadySatisfied
            | TopologyDiagnosticKind::LuksTokenRemoveAlreadySatisfied
            | TopologyDiagnosticKind::MultipathDestroyAlreadySatisfied
            | TopologyDiagnosticKind::MultipathPathAddAlreadySatisfied
            | TopologyDiagnosticKind::MultipathPathRemoveAlreadySatisfied
            | TopologyDiagnosticKind::SwapDeactivateAlreadySatisfied
            | TopologyDiagnosticKind::SwapDestroyAlreadySatisfied
            | TopologyDiagnosticKind::LoopCreateAlreadySatisfied
            | TopologyDiagnosticKind::LoopDetachAlreadySatisfied
            | TopologyDiagnosticKind::MdCreateAlreadySatisfied
            | TopologyDiagnosticKind::MdAssembleAlreadySatisfied
            | TopologyDiagnosticKind::MdStopAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberAddAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberRemoveAlreadySatisfied
            | TopologyDiagnosticKind::MdMemberReplaceAlreadySatisfied
            | TopologyDiagnosticKind::MountAlreadySatisfied
            | TopologyDiagnosticKind::MountOptionsAlreadySatisfied
            | TopologyDiagnosticKind::UnmountAlreadySatisfied
            | TopologyDiagnosticKind::NfsExportAlreadySatisfied
            | TopologyDiagnosticKind::NfsUnexportAlreadySatisfied
            | TopologyDiagnosticKind::PropertyAlreadySatisfied
            | TopologyDiagnosticKind::SnapshotCloneSourceAvailable
            | TopologyDiagnosticKind::SnapshotDestroyAlreadySatisfied
            | TopologyDiagnosticKind::SnapshotRollbackPointAvailable
            | TopologyDiagnosticKind::VdoDestroyAlreadySatisfied
            | TopologyDiagnosticKind::VdoStartAlreadySatisfied
            | TopologyDiagnosticKind::VdoStopAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectCreateAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectDestroyAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectPromoteAlreadySatisfied
            | TopologyDiagnosticKind::ZfsObjectRenameAlreadySatisfied
            | TopologyDiagnosticKind::ZfsPoolCreateAlreadySatisfied
            | TopologyDiagnosticKind::ZfsPoolImportAlreadySatisfied
    )
}

fn rollback_topology_diagnostic_is_data_loss_risk(kind: TopologyDiagnosticKind) -> bool {
    matches!(
        kind,
        TopologyDiagnosticKind::BtrfsSubvolumeDestroyRequired
            | TopologyDiagnosticKind::BtrfsQgroupDestroyRequired
            | TopologyDiagnosticKind::BcacheDetachRequired
            | TopologyDiagnosticKind::LvmCacheDetachRequired
            | TopologyDiagnosticKind::LuksKeyslotRemoveRequired
            | TopologyDiagnosticKind::LuksTokenRemoveRequired
            | TopologyDiagnosticKind::MultipathDestroyRequired
            | TopologyDiagnosticKind::MultipathPathRemoveRequired
            | TopologyDiagnosticKind::SwapDestroyRequired
            | TopologyDiagnosticKind::MdMemberRemoveRequired
            | TopologyDiagnosticKind::SnapshotDestroyRequired
            | TopologyDiagnosticKind::VdoDestroyRequired
            | TopologyDiagnosticKind::ZfsObjectDestroyRequired
    )
}

fn missing_rollback_replay_tools(
    recipe: &RollbackRecipe,
    tool_exists: impl Fn(&str) -> bool,
) -> Vec<String> {
    let mut tools = BTreeSet::new();
    for command in recipe
        .read_only_validation
        .commands
        .iter()
        .chain(recipe.reversible_mutations.commands.iter())
    {
        if let Some(tool) = command.argv.first().filter(|tool| !tool.starts_with('<')) {
            tools.insert(tool.clone());
        }
    }
    tools
        .into_iter()
        .filter(|tool| !tool_exists(tool))
        .collect()
}

fn refused_rollback_report(
    recipe_version: u64,
    source_action_id: &str,
    failed_command: &[String],
    bindings: RollbackReplayBindings,
    refusal_reasons: Vec<String>,
) -> RollbackExecutionReport {
    RollbackExecutionReport {
        status: RollbackExecutionStatus::Refused,
        recipe_version,
        source_action_id: source_action_id.to_string(),
        receipt_binding: RollbackReceiptBinding {
            original_receipt_id: bindings.original_receipt_id,
            source_action_id: source_action_id.to_string(),
            failed_command: failed_command.to_vec(),
            fresh_topology_probe_id: bindings.fresh_topology_probe_id,
            topology_evidence: bindings.topology_evidence,
            topology_payloads: bindings.topology_payloads,
        },
        validation_results: Vec::new(),
        rollback_results: Vec::new(),
        messages: vec!["automatic rollback replay refused before executing commands".to_string()],
        refusal_reasons,
    }
}

fn rollback_receipt_binding(
    recipe: &RollbackRecipe,
    bindings: RollbackReplayBindings,
) -> RollbackReceiptBinding {
    RollbackReceiptBinding {
        original_receipt_id: bindings.original_receipt_id,
        source_action_id: recipe.source_action_id.clone(),
        failed_command: recipe.failed_command.clone(),
        fresh_topology_probe_id: bindings.fresh_topology_probe_id,
        topology_evidence: bindings.topology_evidence,
        topology_payloads: bindings.topology_payloads,
    }
}
