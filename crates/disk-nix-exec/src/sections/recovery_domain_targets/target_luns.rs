fn target_lun_recovery_inspection_commands(
    target: Option<&str>,
    note: &str,
) -> Vec<ExecutionCommand> {
    let mut commands = vec![
        command_vec(["targetcli", "/iscsi", "ls"], false, note),
        command_vec(
            [
                "tgtadm", "--lld", "iscsi", "--mode", "target", "--op", "show",
            ],
            false,
            note,
        ),
        lsscsi_lun_inventory_command(note),
        command_vec(["multipath", "-ll"], false, note),
    ];
    if let Some(target) = target {
        commands.insert(
            1,
            command_vec(
                vec![
                    "targetcli".to_string(),
                    format!("/iscsi/{target}"),
                    "ls".to_string(),
                ],
                false,
                note,
            ),
        );
    }
    commands
}
