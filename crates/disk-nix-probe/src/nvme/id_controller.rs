pub fn normalize_nvme_id_ctrl_json(
    controller: &str,
    bytes: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!(
            "failed to parse nvme id-ctrl JSON for {controller}: {error}"
        ))
    })?;
    let name = controller.trim_start_matches("/dev/");
    let mut node = Node::new(
        nvme_controller_id(name),
        NodeKind::NvmeController,
        name.to_string(),
    )
    .with_path(controller_path(name));

    let serial = field_string(&value, "sn");
    if serial.is_some() {
        node = node.with_identity(Identity {
            serial,
            ..Identity::default()
        });
    }

    for (key, property) in [
        ("mn", "nvme.model"),
        ("fr", "nvme.firmware"),
        ("cntlid", "nvme.controller-id"),
        ("vid", "nvme.id-ctrl.vid"),
        ("ssvid", "nvme.id-ctrl.ssvid"),
        ("rab", "nvme.id-ctrl.rab"),
        ("ieee", "nvme.id-ctrl.ieee"),
        ("cmic", "nvme.id-ctrl.cmic"),
        ("mdts", "nvme.id-ctrl.mdts"),
        ("ver", "nvme.id-ctrl.version"),
        ("rtd3r", "nvme.id-ctrl.rtd3r"),
        ("rtd3e", "nvme.id-ctrl.rtd3e"),
        ("oaes", "nvme.id-ctrl.oaes"),
        ("ctratt", "nvme.id-ctrl.ctratt"),
        ("rrls", "nvme.id-ctrl.rrls"),
        ("cntrltype", "nvme.id-ctrl.controller-type"),
        ("fguid", "nvme.id-ctrl.fguid"),
        ("crdt1", "nvme.id-ctrl.crdt1"),
        ("crdt2", "nvme.id-ctrl.crdt2"),
        ("crdt3", "nvme.id-ctrl.crdt3"),
        ("nvmsr", "nvme.id-ctrl.nvmsr"),
        ("vwci", "nvme.id-ctrl.vwci"),
        ("mec", "nvme.id-ctrl.mec"),
        ("oacs", "nvme.id-ctrl.oacs"),
        ("acl", "nvme.id-ctrl.acl"),
        ("aerl", "nvme.id-ctrl.aerl"),
        ("frmw", "nvme.id-ctrl.frmw"),
        ("lpa", "nvme.id-ctrl.lpa"),
        ("elpe", "nvme.id-ctrl.elpe"),
        ("npss", "nvme.id-ctrl.npss"),
        ("avscc", "nvme.id-ctrl.avscc"),
        ("apsta", "nvme.id-ctrl.apsta"),
        ("wctemp", "nvme.id-ctrl.warning-composite-temp"),
        ("cctemp", "nvme.id-ctrl.critical-composite-temp"),
        ("mtfa", "nvme.id-ctrl.mtfa"),
        ("hmpre", "nvme.id-ctrl.hmpre"),
        ("hmmin", "nvme.id-ctrl.hmmin"),
        ("tnvmcap", "nvme.id-ctrl.total-nvm-capacity"),
        ("unvmcap", "nvme.id-ctrl.unallocated-nvm-capacity"),
        ("rpmbs", "nvme.id-ctrl.rpmbs"),
        ("edstt", "nvme.id-ctrl.edstt"),
        ("dsto", "nvme.id-ctrl.dsto"),
        ("fwug", "nvme.id-ctrl.fwug"),
        ("kas", "nvme.id-ctrl.kas"),
        ("hctma", "nvme.id-ctrl.hctma"),
        ("mntmt", "nvme.id-ctrl.minimum-thermal-management-temp"),
        ("mxtmt", "nvme.id-ctrl.maximum-thermal-management-temp"),
        ("sanicap", "nvme.id-ctrl.sanitize-capabilities"),
        ("hmminds", "nvme.id-ctrl.hmminds"),
        ("hmmaxd", "nvme.id-ctrl.hmmaxd"),
        ("nsetidmax", "nvme.id-ctrl.namespace-set-id-max"),
        ("endgidmax", "nvme.id-ctrl.endurance-group-id-max"),
        ("anatt", "nvme.id-ctrl.ana-transition-time"),
        ("anacap", "nvme.id-ctrl.ana-capabilities"),
        ("anagrpmax", "nvme.id-ctrl.ana-group-max"),
        ("nanagrpid", "nvme.id-ctrl.ana-group-identifiers"),
        ("pels", "nvme.id-ctrl.persistent-event-log-size"),
        ("domainid", "nvme.id-ctrl.domain-id"),
        ("sqes", "nvme.id-ctrl.sqes"),
        ("cqes", "nvme.id-ctrl.cqes"),
        ("maxcmd", "nvme.id-ctrl.maxcmd"),
        ("nn", "nvme.id-ctrl.namespace-count"),
        ("oncs", "nvme.id-ctrl.oncs"),
        ("fuses", "nvme.id-ctrl.fuses"),
        ("fna", "nvme.id-ctrl.fna"),
        ("vwc", "nvme.id-ctrl.volatile-write-cache"),
        ("awun", "nvme.id-ctrl.awun"),
        ("awupf", "nvme.id-ctrl.awupf"),
        ("icsvscc", "nvme.id-ctrl.icsvscc"),
        ("nwpc", "nvme.id-ctrl.nwpc"),
        ("acwu", "nvme.id-ctrl.acwu"),
        ("sgls", "nvme.id-ctrl.sgls"),
        ("mnan", "nvme.id-ctrl.mnan"),
        ("subnqn", "nvme.subsystem"),
    ] {
        if let Some(value) = field_string(&value, key) {
            node = node.with_property(property, value);
        }
    }

    node = node.with_property("nvme.controller", name.to_string());

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
    Ok(graph)
}
