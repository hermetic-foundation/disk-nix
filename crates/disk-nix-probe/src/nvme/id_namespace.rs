pub fn normalize_nvme_id_ns_json(path: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProbeError::Adapter(format!(
            "failed to parse nvme id-ns JSON for {path}: {error}"
        ))
    })?;
    let mut node =
        Node::new(format!("block:{path}"), NodeKind::NvmeNamespace, path).with_path(path);

    let formatted_lba = field_u64(&value, "flbas").map(|value| value & 0xf);
    let lba = formatted_lba.and_then(|index| {
        value
            .get("lbafs")
            .and_then(Value::as_array)?
            .get(index as usize)
    });
    let block_size = lba
        .and_then(|lba| field_u64(lba, "ds"))
        .and_then(|shift| 1_u64.checked_shl(shift as u32));

    let namespace_size = field_u64(&value, "nsze");
    let namespace_capacity = field_u64(&value, "ncap");
    let namespace_used = field_u64(&value, "nuse");
    if let (Some(block_size), Some(namespace_size)) = (block_size, namespace_size) {
        if let Some(size_bytes) = namespace_size.checked_mul(block_size) {
            node = node.with_size_bytes(size_bytes);
        }
    }
    let usage = Usage {
        used_bytes: match (block_size, namespace_used) {
            (Some(block_size), Some(namespace_used)) => namespace_used.checked_mul(block_size),
            _ => None,
        },
        free_bytes: match (block_size, namespace_capacity, namespace_used) {
            (Some(block_size), Some(namespace_capacity), Some(namespace_used)) => {
                namespace_capacity
                    .checked_sub(namespace_used)
                    .and_then(|free_blocks| free_blocks.checked_mul(block_size))
            }
            _ => None,
        },
        allocated_bytes: match (block_size, namespace_capacity) {
            (Some(block_size), Some(namespace_capacity)) => {
                namespace_capacity.checked_mul(block_size)
            }
            _ => None,
        },
    };
    if !usage.is_empty() {
        node = node.with_usage(usage);
    }

    if let Some(nguid) = field_string(&value, "nguid") {
        node = node.with_identity(Identity {
            wwn: Some(nguid.clone()),
            ..Identity::default()
        });
        node = node.with_property("nvme.nguid", nguid);
    }
    if let Some(eui64) = field_string(&value, "eui64") {
        if node.identity.wwn.is_none() {
            node = node.with_identity(Identity {
                wwn: Some(eui64.clone()),
                ..Identity::default()
            });
        }
        node = node.with_property("nvme.eui64", eui64);
    }
    if let Some(index) = formatted_lba {
        node = node.with_property("nvme.formatted-lba-index", index.to_string());
    }
    if let Some(block_size) = block_size {
        node = node.with_property("nvme.formatted-lba-data-size", block_size.to_string());
    }
    if let Some(metadata_size) = lba.and_then(|lba| field_u64(lba, "ms")) {
        node = node.with_property(
            "nvme.formatted-lba-metadata-size",
            metadata_size.to_string(),
        );
    }
    if let Some(relative_performance) = lba.and_then(|lba| field_u64(lba, "rp")) {
        node = node.with_property(
            "nvme.formatted-lba-relative-performance",
            relative_performance.to_string(),
        );
    }

    for key in [
        "nsze", "ncap", "nuse", "nsfeat", "nlbaf", "flbas", "mc", "dpc", "dps", "nmic", "rescap",
        "fpi", "dlfeat", "nawun", "nawupf", "nacwu", "nabsn", "nabo", "nabspf", "noiob", "nvmcap",
    ] {
        if let Some(value) = field_string(&value, key) {
            node = node.with_property(format!("nvme.id-ns.{key}"), value);
        }
    }

    let mut graph = StorageGraph::empty();
    graph.add_node(node);
    Ok(graph)
}
