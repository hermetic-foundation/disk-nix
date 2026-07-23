fn print_inspect(
    output: &mut impl Write,
    graph: &StorageGraph,
    query: &str,
    depth: usize,
) -> io::Result<()> {
    let matches = graph.find_nodes(query);

    if matches.is_empty() {
        writeln!(output, "No storage graph nodes matched '{query}'.")?;
        return Ok(());
    }

    for (index, node) in matches.iter().enumerate() {
        if index > 0 {
            writeln!(output)?;
        }

        writeln!(output, "{} {}", node.kind, node.name)?;
        writeln!(output, "  id: {}", node.id.0)?;
        if let Some(path) = &node.path {
            writeln!(output, "  path: {path}")?;
        }
        if let Some(size_bytes) = node.size_bytes {
            writeln!(output, "  size: {}", human_bytes(Some(size_bytes)))?;
        }
        if let Some(usage) = &node.usage {
            if usage.used_bytes.is_some()
                || usage.free_bytes.is_some()
                || usage.allocated_bytes.is_some()
            {
                writeln!(
                    output,
                    "  usage: used={} free={} allocated={} use={}",
                    human_bytes(usage.used_bytes),
                    human_bytes(usage.free_bytes),
                    human_bytes(usage.allocated_bytes),
                    usage_percent(node)
                )?;
            }
        }

        print_identity(output, node)?;
        print_properties(output, node)?;
        print_relationships(output, graph, node, depth)?;
    }

    Ok(())
}

fn print_identity(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.identity.is_empty() {
        return Ok(());
    }

    writeln!(output, "  identity:")?;
    for (key, value) in [
        ("uuid", node.identity.uuid.as_deref()),
        ("partuuid", node.identity.partuuid.as_deref()),
        ("label", node.identity.label.as_deref()),
        ("serial", node.identity.serial.as_deref()),
        ("wwn", node.identity.wwn.as_deref()),
    ] {
        if let Some(value) = value {
            writeln!(output, "    {key}: {value}")?;
        }
    }
    Ok(())
}

fn print_properties(output: &mut impl Write, node: &Node) -> io::Result<()> {
    if node.properties.is_empty() {
        return Ok(());
    }

    writeln!(output, "  properties:")?;
    for property in &node.properties {
        writeln!(output, "    {}: {}", property.key, property.value)?;
    }
    Ok(())
}

fn print_relationships(
    output: &mut impl Write,
    graph: &StorageGraph,
    node: &Node,
    depth: usize,
) -> io::Result<()> {
    let mut initial_ids = BTreeSet::new();
    initial_ids.insert(node.id.0.clone());
    let subgraph = relationship_subgraph(graph, &initial_ids, depth);
    let edges = subgraph.edges.iter().collect::<Vec<_>>();
    if edges.is_empty() {
        return Ok(());
    }

    writeln!(output, "  relationships:")?;
    for edge in edges {
        if depth <= 1 {
            let direction = if edge.from == node.id { "out" } else { "in" };
            let other_id = if edge.from == node.id {
                &edge.to
            } else {
                &edge.from
            };
            let other_name = graph
                .nodes
                .iter()
                .find(|candidate| &candidate.id == other_id)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(other_id.0.as_str());

            writeln!(
                output,
                "    {direction} {} {} ({})",
                edge.relationship, other_id.0, other_name
            )?;
        } else {
            let from_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.from)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.from.0.as_str());
            let to_name = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == edge.to)
                .map(|candidate| candidate.name.as_str())
                .unwrap_or(edge.to.0.as_str());

            writeln!(
                output,
                "    {} ({}) {} {} ({})",
                edge.from.0, from_name, edge.relationship, edge.to.0, to_name
            )?;
        }
    }

    Ok(())
}

fn print_plan(output: &mut impl Write, plan: &Plan) -> io::Result<()> {
    writeln!(
        output,
        "Plan: {} actions, {} offline required, {} destructive, {} potential data loss, {} unsupported",
        plan.summary.action_count,
        plan.summary.offline_required_count,
        plan.summary.destructive_count,
        plan.summary.potential_data_loss_count,
        plan.summary.unsupported_count
    )?;

    for action in &plan.actions {
        writeln!(
            output,
            "- {:?} {:?}: {}",
            action.risk, action.operation, action.description
        )?;

        if let Some(advice) = &action.advice {
            writeln!(output, "  advice: {}", advice.summary)?;
            for alternative in &advice.alternatives {
                writeln!(output, "  alternative: {alternative}")?;
            }
        }
    }

    if let Some(comparison) = &plan.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    Ok(())
}

fn print_topology_comparison(
    output: &mut impl Write,
    comparison: &TopologyComparison,
) -> io::Result<()> {
    writeln!(
        output,
        "Topology comparison: {} actions, {} matched, {} missing, {} size notes, {} type conflicts, {} already satisfied, {} suppressed, {} graph dependency conflicts",
        comparison.summary.action_count,
        comparison.summary.matched_count,
        comparison.summary.missing_count,
        comparison.summary.size_diagnostic_count,
        comparison.summary.type_conflict_count,
        comparison.summary.already_satisfied_count,
        comparison.summary.suppressed_action_count,
        comparison.summary.graph_dependency_conflict_count
    )?;

    for diagnostic in &comparison.diagnostics {
        let level = match diagnostic.level {
            TopologyDiagnosticLevel::Info => "info",
            TopologyDiagnosticLevel::Warning => "warning",
        };
        writeln!(
            output,
            "  {level}: {:?} {}: {}",
            diagnostic.kind, diagnostic.action_id, diagnostic.message
        )?;
    }

    Ok(())
}

fn print_execution_report(
    output: &mut impl Write,
    report: &ExecutionReport,
    execute: bool,
) -> io::Result<()> {
    writeln!(
        output,
        "Apply policy: {} allowed, {} blocked",
        report.apply.allowed_count, report.apply.blocked_count
    )?;
    writeln!(output, "mode: {:?}", report.apply.policy.mode)?;
    writeln!(output, "status: {:?}", report.status)?;
    writeln!(output, "execute requested: {execute}")?;
    if let Some(comparison) = &report.topology_comparison {
        print_topology_comparison(output, comparison)?;
    }

    if report.apply.blocked.is_empty() {
        writeln!(output, "No policy blocks detected.")?;
        for message in &report.messages {
            writeln!(output, "{message}")?;
        }
        if !report.command_plan.is_empty() {
            writeln!(
                output,
                "Command summary: {} steps, {} commands, {} mutating, {} manual review, {} ready, {} need size, {} need implementation, {} manual only",
                report.command_summary.step_count,
                report.command_summary.command_count,
                report.command_summary.mutating_count,
                report.command_summary.manual_review_count,
                report.command_summary.ready_count,
                report.command_summary.needs_desired_size_count,
                report.command_summary.needs_domain_implementation_count,
                report.command_summary.manual_only_count
            )?;
            writeln!(output, "Command plan:")?;
            if !report.tool_requirements.is_empty() {
                writeln!(output, "Tool requirements:")?;
                for requirement in &report.tool_requirements {
                    writeln!(
                        output,
                        "- {}: {} commands, {} mutating, {} verification, phases {:?}, availability {:?}",
                        requirement.tool,
                        requirement.command_count,
                        requirement.mutating_count,
                        requirement.verification_count,
                        requirement.phases,
                        requirement.availability
                    )?;
                    writeln!(output, "  {}", requirement.message)?;
                    for remediation in &requirement.remediation {
                        writeln!(output, "  - {remediation}")?;
                    }
                }
            }
            for step in &report.command_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                if step.requires_manual_review {
                    writeln!(output, "  manual review required")?;
                }
                for command in &step.commands {
                    let mutation = if command.mutates {
                        "mutating"
                    } else {
                        "read-only"
                    };
                    writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                    writeln!(output, "    readiness: {:?}", command.readiness)?;
                    if !command.unresolved_inputs.is_empty() {
                        writeln!(
                            output,
                            "    unresolved: {}",
                            command.unresolved_inputs.join(", ")
                        )?;
                    }
                    writeln!(output, "    {}", command.note)?;
                }
                for note in &step.notes {
                    writeln!(output, "  note: {note}")?;
                }
            }
        }
        if !report.execution_results.is_empty() {
            writeln!(
                output,
                "Execution results: {} command(s)",
                report.execution_results.len()
            )?;
            for result in &report.execution_results {
                let status = if result.success { "ok" } else { "failed" };
                writeln!(
                    output,
                    "- {:?} {} {}",
                    result.phase,
                    status,
                    result.argv.join(" ")
                )?;
                if let Some(status_code) = result.status_code {
                    writeln!(output, "  exit: {status_code}")?;
                }
                if !result.stdout.is_empty() {
                    writeln!(output, "  stdout: {}", result.stdout.trim_end())?;
                }
                if !result.stderr.is_empty() {
                    writeln!(output, "  stderr: {}", result.stderr.trim_end())?;
                }
            }
        }
        if !report.verification_plan.is_empty() {
            writeln!(
                output,
                "Verification summary: {} steps, {} read-only commands, {} checks",
                report.verification_summary.step_count,
                report.verification_summary.command_count,
                report.verification_summary.check_count
            )?;
            writeln!(output, "Verification plan:")?;
            for step in &report.verification_plan {
                writeln!(
                    output,
                    "- {:?} {:?} {}",
                    step.risk, step.operation, step.action_id
                )?;
                for command in &step.commands {
                    writeln!(output, "  read-only: {}", command.argv.join(" "))?;
                    writeln!(output, "    {}", command.note)?;
                }
                for check in &step.checks {
                    writeln!(output, "  check: {check}")?;
                }
            }
        }
    } else {
        writeln!(
            output,
            "Blocked summary: {} offline required, {} destructive, {} potential data loss, {} unsupported",
            report.apply.blocked_summary.offline_required_count,
            report.apply.blocked_summary.destructive_count,
            report.apply.blocked_summary.potential_data_loss_count,
            report.apply.blocked_summary.unsupported_count
        )?;
        writeln!(output, "Blocked actions:")?;
        for blocked in &report.apply.blocked {
            writeln!(
                output,
                "- {:?} {:?} {}: {}",
                blocked.risk, blocked.operation, blocked.id, blocked.reason
            )?;
        }
    }

    if !report.recovery_actions.is_empty() {
        writeln!(output, "Recovery actions:")?;
        for action in &report.recovery_actions {
            writeln!(output, "- {:?}: {}", action.kind, action.summary)?;
            for command in &action.commands {
                let mutation = if command.mutates {
                    "mutating"
                } else {
                    "read-only"
                };
                writeln!(output, "  {mutation}: {}", command.argv.join(" "))?;
                writeln!(output, "    readiness: {:?}", command.readiness)?;
                writeln!(output, "    {}", command.note)?;
            }
            for note in &action.notes {
                writeln!(output, "  note: {note}")?;
            }
        }
    }

    Ok(())
}
