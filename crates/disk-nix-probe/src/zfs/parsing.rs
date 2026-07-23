fn parse_zpools(bytes: &[u8]) -> Result<Vec<ZpoolRow>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read zpool output: {error}")))?;
    let mut rows = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            return Err(ProbeError::Adapter(format!(
                "zpool row has {} fields, expected at least 5: {line}",
                fields.len()
            )));
        }

        rows.push(ZpoolRow {
            name: fields[0].to_string(),
            size: parse_u64_field(fields[1]),
            allocated: parse_u64_field(fields[2]),
            free: parse_u64_field(fields[3]),
            health: nonempty(fields[4]),
            capacity: fields.get(5).and_then(|value| nonempty_dash(value)),
            dedupratio: fields.get(6).and_then(|value| nonempty_dash(value)),
            fragmentation: fields.get(7).and_then(|value| nonempty_dash(value)),
            altroot: fields.get(8).and_then(|value| nonempty_dash(value)),
        });
    }

    Ok(rows)
}

fn parse_zpool_status(bytes: &[u8]) -> Result<Vec<ZpoolStatus>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zpool status output: {error}"))
    })?;
    let mut pools = Vec::new();
    let mut current: Option<ZpoolStatus> = None;
    let mut in_config = false;
    let mut role = "data".to_string();
    let mut stack: Vec<(usize, String)> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = trimmed.strip_prefix("pool:").map(str::trim) {
            if let Some(pool) = current.take() {
                pools.push(pool);
            }
            current = Some(ZpoolStatus {
                name: name.to_string(),
                state: None,
                status: None,
                action: None,
                scan: None,
                errors: None,
                read_errors: None,
                write_errors: None,
                checksum_errors: None,
                vdevs: Vec::new(),
            });
            in_config = false;
            role = "data".to_string();
            stack.clear();
            continue;
        }

        let Some(pool) = &mut current else {
            continue;
        };

        if let Some(state) = trimmed.strip_prefix("state:").map(str::trim) {
            pool.state = nonempty(state);
            continue;
        }
        if let Some(status) = trimmed.strip_prefix("status:").map(str::trim) {
            pool.status = nonempty(status);
            continue;
        }
        if let Some(action) = trimmed.strip_prefix("action:").map(str::trim) {
            pool.action = nonempty(action);
            continue;
        }
        if let Some(scan) = trimmed.strip_prefix("scan:").map(str::trim) {
            pool.scan = nonempty(scan);
            continue;
        }
        if trimmed == "config:" {
            in_config = true;
            continue;
        }
        if let Some(errors) = trimmed.strip_prefix("errors:").map(str::trim) {
            pool.errors = nonempty(errors);
            in_config = false;
            continue;
        }
        if !in_config || trimmed.starts_with("NAME ") {
            continue;
        }
        if matches!(trimmed, "logs" | "cache" | "spares" | "special" | "dedup") {
            role = trimmed.to_string();
            stack.clear();
            continue;
        }

        let Some(vdev) = parse_vdev_line(&pool.name, &role, line, &mut stack) else {
            continue;
        };
        if vdev.name == pool.name {
            pool.read_errors = vdev.read_errors;
            pool.write_errors = vdev.write_errors;
            pool.checksum_errors = vdev.checksum_errors;
        } else {
            pool.vdevs.push(vdev);
        }
    }

    if let Some(pool) = current {
        pools.push(pool);
    }

    Ok(pools)
}

fn parse_zpool_properties(bytes: &[u8]) -> Result<Vec<ZpoolProperty>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zpool get output: {error}"))
    })?;
    let mut properties = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            return Err(ProbeError::Adapter(format!(
                "zpool get row has {} fields, expected at least 3: {line}",
                fields.len()
            )));
        }

        let Some(value) = nonempty_dash(fields[2]) else {
            continue;
        };
        properties.push(ZpoolProperty {
            pool: fields[0].to_string(),
            property: fields[1].to_string(),
            value,
        });
    }

    Ok(properties)
}

fn parse_vdev_line(
    pool_name: &str,
    role: &str,
    line: &str,
    stack: &mut Vec<(usize, String)>,
) -> Option<ZpoolVdev> {
    let indent = line
        .chars()
        .take_while(|character| *character == ' ')
        .count();
    let fields: Vec<&str> = line.split_whitespace().collect();
    let name = fields.first()?.to_string();
    let state = fields.get(1).map(|value| (*value).to_string());
    let parent = stack
        .iter()
        .rev()
        .find(|(parent_indent, _)| *parent_indent < indent)
        .map(|(_, parent)| parent.clone());

    stack.retain(|(parent_indent, _)| *parent_indent < indent);
    stack.push((indent, name.clone()));

    Some(ZpoolVdev {
        device_path: name.starts_with("/dev/").then(|| name.clone()),
        name: name.clone(),
        role: if name == pool_name {
            "pool".to_string()
        } else {
            role.to_string()
        },
        parent,
        state,
        read_errors: fields.get(2).map(|value| (*value).to_string()),
        write_errors: fields.get(3).map(|value| (*value).to_string()),
        checksum_errors: fields.get(4).map(|value| (*value).to_string()),
    })
}

fn parse_datasets(bytes: &[u8]) -> Result<Vec<ZfsRow>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read zfs output: {error}")))?;
    let mut rows = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            return Err(ProbeError::Adapter(format!(
                "zfs row has {} fields, expected at least 7: {line}",
                fields.len()
            )));
        }

        rows.push(ZfsRow {
            name: fields[0].to_string(),
            kind: fields[1].to_string(),
            used: parse_u64_field(fields[2]),
            available: parse_u64_field(fields[3]),
            referenced: parse_u64_field(fields[4]),
            mountpoint: nonempty_dash(fields[5]),
            origin: nonempty_dash(fields[6]),
            userrefs: fields.get(7).and_then(|value| nonempty_dash(value)),
            compression: fields.get(8).and_then(|value| nonempty_dash(value)),
            quota: fields.get(9).and_then(|value| nonempty_dash(value)),
            reservation: fields.get(10).and_then(|value| nonempty_dash(value)),
            encryption: fields.get(11).and_then(|value| nonempty_dash(value)),
            keystatus: fields.get(12).and_then(|value| nonempty_dash(value)),
            volsize: fields.get(13).and_then(|value| nonempty_dash(value)),
            recordsize: fields.get(14).and_then(|value| nonempty_dash(value)),
            dedup: fields.get(15).and_then(|value| nonempty_dash(value)),
            checksum: fields.get(16).and_then(|value| nonempty_dash(value)),
            copies: fields.get(17).and_then(|value| nonempty_dash(value)),
            sync: fields.get(18).and_then(|value| nonempty_dash(value)),
            primarycache: fields.get(19).and_then(|value| nonempty_dash(value)),
            secondarycache: fields.get(20).and_then(|value| nonempty_dash(value)),
            atime: fields.get(21).and_then(|value| nonempty_dash(value)),
            relatime: fields.get(22).and_then(|value| nonempty_dash(value)),
            snapdir: fields.get(23).and_then(|value| nonempty_dash(value)),
            acltype: fields.get(24).and_then(|value| nonempty_dash(value)),
            xattr: fields.get(25).and_then(|value| nonempty_dash(value)),
        });
    }

    Ok(rows)
}

fn parse_zfs_holds(bytes: &[u8]) -> Result<Vec<ZfsHold>, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read zfs holds output: {error}"))
    })?;
    let mut holds = Vec::new();

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 2 {
            return Err(ProbeError::Adapter(format!(
                "zfs holds row has {} fields, expected at least 2: {line}",
                fields.len()
            )));
        }
        holds.push(ZfsHold {
            snapshot: fields[0].to_string(),
            tag: fields[1].to_string(),
            timestamp: fields.get(2).and_then(|value| nonempty_dash(value)),
        });
    }

    Ok(holds)
}
