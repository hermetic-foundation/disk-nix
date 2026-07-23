fn add_device(graph: &mut StorageGraph, device: NvmeDevice) {
    let Some(path) = device.device_path else {
        return;
    };
    let id = format!("block:{path}");
    let mut node = Node::new(id, NodeKind::NvmeNamespace, path.clone()).with_path(path.clone());

    let size_bytes = device.physical_size.or(device.namespace_capacity);
    if let Some(size_bytes) = size_bytes {
        node = node.with_size_bytes(size_bytes);
    }

    let usage = Usage {
        used_bytes: device.used_bytes,
        free_bytes: match (size_bytes, device.used_bytes) {
            (Some(size), Some(used)) => size.checked_sub(used),
            _ => None,
        },
        allocated_bytes: device.used_bytes,
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    let controller = device.controller.clone();
    let controller_id = device.controller_id;
    let serial = device.serial_number.clone();
    let namespace_uuid = device.namespace_uuid.clone();
    let namespace_wwn = device.nguid.clone().or_else(|| device.eui64.clone());
    let model = device.model_number.clone();
    let product = device.product_name.clone();
    let firmware = device.firmware.clone();
    let subsystem = device.subsystem.clone();
    let address = device.address.clone();
    let transport = device.transport.clone();

    if serial.is_some() || namespace_uuid.is_some() || namespace_wwn.is_some() {
        node = node.with_identity(Identity {
            uuid: namespace_uuid.clone(),
            serial: serial.clone(),
            wwn: namespace_wwn,
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("nvme.generic-path", device.generic),
        ("nvme.model", device.model_number),
        ("nvme.product", device.product_name),
        ("nvme.firmware", device.firmware),
        ("nvme.index", device.index.map(|value| value.to_string())),
        (
            "nvme.namespace",
            device.namespace.map(|value| value.to_string()),
        ),
        (
            "nvme.namespace-id",
            device.namespace_id.map(|value| value.to_string()),
        ),
        ("nvme.namespace-uuid", device.namespace_uuid),
        ("nvme.eui64", device.eui64),
        ("nvme.nguid", device.nguid),
        ("nvme.subsystem", device.subsystem),
        ("nvme.controller", device.controller),
        ("nvme.address", device.address),
        ("nvme.transport", device.transport),
        (
            "nvme.controller-id",
            device.controller_id.map(|value| value.to_string()),
        ),
        (
            "nvme.namespace-capacity",
            device.namespace_capacity.map(|value| value.to_string()),
        ),
        ("nvme.lba-format", device.lba_format),
        (
            "nvme.maximum-lba",
            device.maximum_lba.map(|value| value.to_string()),
        ),
        (
            "nvme.sector-size",
            device.sector_size.map(|value| value.to_string()),
        ),
        ("nvme.ana-state", device.ana_state),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);

    if let Some(controller) = controller {
        add_controller(
            graph,
            &controller,
            ControllerSummary {
                serial,
                model,
                product,
                firmware,
                subsystem,
                address,
                transport,
                controller_id,
            },
        );
        graph.add_edge(Edge::new(
            nvme_controller_id(&controller),
            format!("block:{path}"),
            Relationship::Contains,
        ));
    }
}

struct ControllerSummary {
    serial: Option<String>,
    model: Option<String>,
    product: Option<String>,
    firmware: Option<String>,
    subsystem: Option<String>,
    address: Option<String>,
    transport: Option<String>,
    controller_id: Option<u64>,
}

fn add_controller(graph: &mut StorageGraph, controller: &str, summary: ControllerSummary) {
    let name = controller.trim_start_matches("/dev/");
    let mut node = Node::new(
        nvme_controller_id(name),
        NodeKind::NvmeController,
        name.to_string(),
    )
    .with_path(controller_path(name));

    if summary.serial.is_some() {
        node = node.with_identity(Identity {
            serial: summary.serial,
            ..Identity::default()
        });
    }

    for (key, value) in [
        ("nvme.controller", Some(name.to_string())),
        ("nvme.model", summary.model),
        ("nvme.product", summary.product),
        ("nvme.firmware", summary.firmware),
        ("nvme.subsystem", summary.subsystem),
        ("nvme.address", summary.address),
        ("nvme.transport", summary.transport),
        (
            "nvme.controller-id",
            summary.controller_id.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }

    graph.add_node(node);
}

fn add_subsystem_path(
    graph: &mut StorageGraph,
    subsystem_id: &str,
    subsystem_name: &str,
    subsystem_nqn: Option<&str>,
    path: &Value,
) {
    let Some(controller) = field_string_any(
        path,
        &[
            "Name",
            "Controller",
            "ControllerName",
            "Device",
            "ControllerPath",
        ],
    ) else {
        return;
    };
    let name = controller.trim_start_matches("/dev/").to_string();
    let mut node = Node::new(
        nvme_controller_id(&name),
        NodeKind::NvmeController,
        name.clone(),
    )
    .with_path(controller_path(&name))
    .with_property("nvme.controller", name.clone())
    .with_property("nvme.subsystem", subsystem_name.to_string());

    if let Some(nqn) = subsystem_nqn {
        node = node.with_property("nvme.subsystem-nqn", nqn.to_string());
    }
    for (key, property) in [
        ("Transport", "nvme.transport"),
        ("TrType", "nvme.transport"),
        ("Address", "nvme.address"),
        ("TransportAddress", "nvme.address"),
        ("TRADDR", "nvme.traddr"),
        ("TRSVCID", "nvme.trsvcid"),
        ("HostTRADDR", "nvme.host-traddr"),
        ("HostIface", "nvme.host-iface"),
        ("State", "nvme.path-state"),
        ("ANAState", "nvme.ana-state"),
    ] {
        if let Some(value) = field_string(path, key) {
            node = node.with_property(property, value);
        }
    }

    graph.add_node(node);
    graph.add_edge(Edge::new(
        subsystem_id.to_string(),
        nvme_controller_id(&name),
        Relationship::Contains,
    ));

    for namespace in subsystem_namespaces(path) {
        if let Some(namespace_path) = field_string_any(
            namespace,
            &["Name", "Path", "DevicePath", "NamespacePath", "BlockDevice"],
        ) {
            let mut node = Node::new(
                format!("block:{namespace_path}"),
                NodeKind::NvmeNamespace,
                namespace_path.clone(),
            )
            .with_path(namespace_path.clone())
            .with_property("nvme.controller", name.clone())
            .with_property("nvme.subsystem", subsystem_name.to_string());
            if let Some(nqn) = subsystem_nqn {
                node = node.with_property("nvme.subsystem-nqn", nqn.to_string());
            }
            if let Some(nsid) = field_string_any(namespace, &["NSID", "Namespace", "NameSpace"]) {
                node = node.with_property("nvme.namespace-id", nsid);
            }
            graph.add_node(node);
            graph.add_edge(Edge::new(
                nvme_controller_id(&name),
                format!("block:{namespace_path}"),
                Relationship::Contains,
            ));
        }
    }
}
