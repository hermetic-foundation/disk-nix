use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CryptStatus {
    mapper_path: String,
    active: Option<bool>,
    in_use: Option<bool>,
    backing_device: Option<String>,
    sector_size: Option<u64>,
    sector_count: Option<u64>,
    properties: Vec<(String, String)>,
    uuid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LuksDump {
    device_path: String,
    uuid: Option<String>,
    label: Option<String>,
    keyslots: Vec<String>,
    tokens: Vec<String>,
    digests: Vec<String>,
    properties: Vec<(String, String)>,
}

pub fn normalize_cryptsetup_status(
    mapper_path: &str,
    bytes: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let status = parse_status(mapper_path, bytes)?;
    let mut graph = StorageGraph::empty();
    add_status(&mut graph, status);
    Ok(graph)
}

pub fn normalize_luks_dump(device_path: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let dump = parse_luks_dump(device_path, bytes)?;
    let mut graph = StorageGraph::empty();
    add_luks_dump(&mut graph, dump);
    Ok(graph)
}

fn parse_status(mapper_path: &str, bytes: &[u8]) -> Result<CryptStatus, ProbeError> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProbeError::Adapter(format!("failed to read cryptsetup status: {error}"))
    })?;
    let mut status = CryptStatus {
        mapper_path: mapper_path.to_string(),
        active: None,
        in_use: None,
        backing_device: None,
        sector_size: None,
        sector_count: None,
        properties: Vec::new(),
        uuid: None,
    };

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if index == 0 {
            parse_header(trimmed, &mut status);
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        match key {
            "device" => status.backing_device = Some(value.to_string()),
            "sector size" => status.sector_size = parse_leading_u64(value),
            "size" => status.sector_count = parse_leading_u64(value),
            "uuid" | "UUID" => status.uuid = Some(value.to_string()),
            _ => status.properties.push((
                format!("cryptsetup.{}", normalize_key(key)),
                value.to_string(),
            )),
        }
    }

    Ok(status)
}

fn parse_luks_dump(device_path: &str, bytes: &[u8]) -> Result<LuksDump, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read luksDump output: {error}")))?;
    let mut dump = LuksDump {
        device_path: device_path.to_string(),
        uuid: None,
        label: None,
        keyslots: Vec::new(),
        tokens: Vec::new(),
        digests: Vec::new(),
        properties: Vec::new(),
    };
    let mut section: Option<&str> = None;
    let mut current_keyslot: Option<String> = None;
    let mut current_token: Option<String> = None;
    let mut current_digest: Option<String> = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "LUKS header information" {
            continue;
        }

        if !line.starts_with(char::is_whitespace) && trimmed.ends_with(':') {
            section = Some(trimmed.trim_end_matches(':'));
            current_keyslot = None;
            current_token = None;
            current_digest = None;
            continue;
        }

        match section {
            Some("Keyslots") => parse_keyslot_line(trimmed, &mut dump, &mut current_keyslot),
            Some("Tokens") => parse_token_line(trimmed, &mut dump, &mut current_token),
            Some("Digests") => parse_digest_line(trimmed, &mut dump, &mut current_digest),
            Some("Data segments") => parse_data_segment_line(trimmed, &mut dump),
            _ => parse_luks_header_line(trimmed, &mut dump),
        }
    }

    if !dump.keyslots.is_empty() {
        dump.properties.push((
            "cryptsetup.luks-keyslots".to_string(),
            dump.keyslots.join(","),
        ));
        dump.properties.push((
            "cryptsetup.luks-keyslot-count".to_string(),
            dump.keyslots.len().to_string(),
        ));
    }
    if !dump.tokens.is_empty() {
        dump.properties
            .push(("cryptsetup.luks-tokens".to_string(), dump.tokens.join(",")));
        dump.properties.push((
            "cryptsetup.luks-token-count".to_string(),
            dump.tokens.len().to_string(),
        ));
    }
    if !dump.digests.is_empty() {
        dump.properties.push((
            "cryptsetup.luks-digests".to_string(),
            dump.digests.join(","),
        ));
        dump.properties.push((
            "cryptsetup.luks-digest-count".to_string(),
            dump.digests.len().to_string(),
        ));
    }

    Ok(dump)
}

fn parse_header(line: &str, status: &mut CryptStatus) {
    if let Some((path, rest)) = line.split_once(" is ") {
        status.mapper_path = path.to_string();
        status.active = Some(rest.starts_with("active"));
        status.in_use = Some(rest.contains("in use"));
    }
}

fn parse_luks_header_line(line: &str, dump: &mut LuksDump) {
    let Some((key, value)) = split_luks_key_value(line) else {
        return;
    };
    match key {
        "UUID" => dump.uuid = Some(value.to_string()),
        "Label" => dump.label = Some(value.to_string()),
        "Version" => dump
            .properties
            .push(("cryptsetup.luks-version".to_string(), value.to_string())),
        "Epoch" => dump
            .properties
            .push(("cryptsetup.luks-epoch".to_string(), value.to_string())),
        "Metadata area" => dump.properties.push((
            "cryptsetup.luks-metadata-area".to_string(),
            value.to_string(),
        )),
        "Keyslots area" => dump.properties.push((
            "cryptsetup.luks-keyslots-area".to_string(),
            value.to_string(),
        )),
        "Subsystem" => dump
            .properties
            .push(("cryptsetup.luks-subsystem".to_string(), value.to_string())),
        "Flags" => dump
            .properties
            .push(("cryptsetup.luks-flags".to_string(), value.to_string())),
        _ => {}
    }
}

fn parse_keyslot_line(line: &str, dump: &mut LuksDump, current_keyslot: &mut Option<String>) {
    if let Some((slot, kind)) = numbered_section_item(line) {
        dump.keyslots.push(slot.to_string());
        dump.properties.push((
            format!("cryptsetup.luks-keyslot-{slot}-type"),
            kind.to_string(),
        ));
        *current_keyslot = Some(slot.to_string());
        return;
    }

    let Some(slot) = current_keyslot.as_deref() else {
        return;
    };
    let Some((key, value)) = split_luks_key_value(line) else {
        return;
    };
    match key {
        "Key" | "Priority" | "Cipher" | "Cipher key" | "PBKDF" | "Time cost" | "Memory"
        | "Threads" | "Salt" | "AF stripes" | "Area offset" | "Area length" | "Digest ID"
        | "Hash" => dump.properties.push((
            format!("cryptsetup.luks-keyslot-{slot}-{}", normalize_key(key)),
            value.to_string(),
        )),
        _ => {}
    }
}

fn parse_token_line(line: &str, dump: &mut LuksDump, current_token: &mut Option<String>) {
    if let Some((token, kind)) = numbered_section_item(line) {
        dump.tokens.push(token.to_string());
        dump.properties.push((
            format!("cryptsetup.luks-token-{token}-type"),
            kind.to_string(),
        ));
        *current_token = Some(token.to_string());
        return;
    }

    let Some(token) = current_token.as_deref() else {
        return;
    };
    let Some((key, value)) = split_luks_key_value(line) else {
        return;
    };
    dump.properties.push((
        format!("cryptsetup.luks-token-{token}-{}", normalize_key(key)),
        value.to_string(),
    ));
}

fn parse_digest_line(line: &str, dump: &mut LuksDump, current_digest: &mut Option<String>) {
    if let Some((digest, kind)) = numbered_section_item(line) {
        dump.digests.push(digest.to_string());
        dump.properties.push((
            format!("cryptsetup.luks-digest-{digest}-type"),
            kind.to_string(),
        ));
        *current_digest = Some(digest.to_string());
        return;
    }

    let Some(digest) = current_digest.as_deref() else {
        return;
    };
    let Some((key, value)) = split_luks_key_value(line) else {
        return;
    };
    match key {
        "Hash" | "Iterations" | "Salt" | "Digest" => dump.properties.push((
            format!("cryptsetup.luks-digest-{digest}-{}", normalize_key(key)),
            value.to_string(),
        )),
        _ => {}
    }
}

fn parse_data_segment_line(line: &str, dump: &mut LuksDump) {
    let Some((key, value)) = split_luks_key_value(line) else {
        return;
    };
    match key {
        "offset" | "length" | "cipher" | "sector" => dump.properties.push((
            format!("cryptsetup.luks-data-{}", normalize_key(key)),
            value.to_string(),
        )),
        _ => {}
    }
}

fn split_luks_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some((key.trim(), value))
    }
}

fn numbered_section_item(line: &str) -> Option<(&str, &str)> {
    let (number, value) = line.split_once(':')?;
    if number.chars().all(|character| character.is_ascii_digit()) {
        Some((number, value.trim()))
    } else {
        None
    }
}

fn add_status(graph: &mut StorageGraph, status: CryptStatus) {
    let id = format!("block:{}", status.mapper_path);
    let name = status
        .mapper_path
        .strip_prefix("/dev/mapper/")
        .unwrap_or(&status.mapper_path)
        .to_string();
    let mut node =
        Node::new(id.clone(), NodeKind::LuksContainer, name).with_path(status.mapper_path);

    if let Some(size_bytes) = status
        .sector_count
        .zip(status.sector_size)
        .map(|(sectors, sector_size)| sectors.saturating_mul(sector_size))
    {
        node = node.with_size_bytes(size_bytes);
    }

    if let Some(uuid) = status.uuid {
        node = node.with_identity(Identity {
            uuid: Some(uuid),
            ..Identity::default()
        });
    }

    for (key, value) in [
        (
            "cryptsetup.active",
            status.active.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.in-use",
            status.in_use.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.sector-size",
            status.sector_size.map(|value| value.to_string()),
        ),
        (
            "cryptsetup.sector-count",
            status.sector_count.map(|value| value.to_string()),
        ),
    ] {
        if let Some(value) = value {
            node = node.with_property(key, value);
        }
    }
    for (key, value) in status.properties {
        node = node.with_property(key, value);
    }

    graph.add_node(node);

    if let Some(backing_device) = status.backing_device {
        let backing_id = format!("block:{backing_device}");
        graph.add_node(
            Node::new(
                backing_id.clone(),
                NodeKind::DeviceMapper,
                backing_device.clone(),
            )
            .with_path(backing_device),
        );
        graph.add_edge(Edge::new(backing_id, id, Relationship::Backs));
    }
}

fn add_luks_dump(graph: &mut StorageGraph, dump: LuksDump) {
    let id = format!("block:{}", dump.device_path);
    let name = dump
        .device_path
        .rsplit('/')
        .next()
        .unwrap_or(&dump.device_path)
        .to_string();
    let mut node = Node::new(id, NodeKind::LuksContainer, name).with_path(dump.device_path);

    let identity = Identity {
        uuid: dump.uuid,
        label: dump.label,
        ..Identity::default()
    };
    if !identity.is_empty() {
        node = node.with_identity(identity);
    }

    for (key, value) in dump.properties {
        node = node.with_property(key, value);
    }

    graph.add_node(node);
}

fn parse_leading_u64(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|number| number.parse().ok())
}

fn normalize_key(key: &str) -> String {
    key.trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| match character {
            'a'..='z' | '0'..='9' => character,
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const STATUS: &[u8] = br#"
/dev/mapper/cryptroot is active and is in use.
  type:    LUKS2
  cipher:  aes-xts-plain64
  keysize: 512 bits
  key location: keyring
  device:  /dev/nvme0n1p2
  sector size:  512
  offset:  32768 sectors
  size:    2097152 sectors
  mode:    read/write
"#;

    const LUKS_DUMP: &[u8] = br#"
LUKS header information
Version:        2
Epoch:          7
Metadata area:  16384 [bytes]
Keyslots area:  16744448 [bytes]
UUID:           luks-uuid
Label:          root-crypt
Subsystem:      (no subsystem)
Flags:          allow-discards

Data segments:
  0: crypt
        offset: 32768 [bytes]
        length: (whole device)
        cipher: aes-xts-plain64
        sector: 4096 [bytes]

Keyslots:
  0: luks2
        Key:        512 bits
        Priority:   normal
        Cipher:     aes-xts-plain64
        Cipher key: 512 bits
        PBKDF:      argon2id
        Time cost:  4
        Memory:     1048576
        Threads:    4
        Salt:       00 11 22 33
        AF stripes: 4000
        Area offset:32768 [bytes]
        Area length:258048 [bytes]
        Digest ID:  0
  1: luks2
        Priority:   ignored

Tokens:
  0: systemd-tpm2
        Keyslot:    0
        Keyslots:   0
        TPM2 PCRs:  0+7
        TPM2 Hash:  sha256

Digests:
  0: pbkdf2
        Hash:       sha256
        Iterations: 1000
        Salt:       aa bb cc dd
        Digest:     ee ff 00 11
"#;

    #[test]
    fn normalizes_cryptsetup_status() {
        let graph =
            normalize_cryptsetup_status("/dev/mapper/cryptroot", STATUS).expect("status parses");
        let container = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::LuksContainer && node.name == "cryptroot")
            .expect("container node should exist");

        assert_eq!(container.path.as_deref(), Some("/dev/mapper/cryptroot"));
        assert_eq!(container.size_bytes, Some(1_073_741_824));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.cipher" && property.value == "aes-xts-plain64"
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/nvme0n1p2"
                && edge.to.0 == "block:/dev/mapper/cryptroot"
                && edge.relationship == Relationship::Backs
        }));
    }

    #[test]
    fn normalizes_luks_dump_header_metadata() {
        let graph = normalize_luks_dump("/dev/nvme0n1p2", LUKS_DUMP).expect("dump parses");
        let container = graph
            .nodes
            .iter()
            .find(|node| {
                node.kind == NodeKind::LuksContainer
                    && node.path.as_deref() == Some("/dev/nvme0n1p2")
            })
            .expect("container node should exist");

        assert_eq!(container.identity.uuid.as_deref(), Some("luks-uuid"));
        assert_eq!(container.identity.label.as_deref(), Some("root-crypt"));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-version" && property.value == "2"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-count" && property.value == "2"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-token-0-type" && property.value == "systemd-tpm2"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-token-0-keyslot" && property.value == "0"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-token-0-keyslots" && property.value == "0"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-token-0-tpm2-pcrs" && property.value == "0+7"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-token-0-tpm2-hash" && property.value == "sha256"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-0-pbkdf" && property.value == "argon2id"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-0-af-stripes" && property.value == "4000"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-0-area-offset"
                && property.value == "32768 [bytes]"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-0-area-length"
                && property.value == "258048 [bytes]"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-keyslot-0-digest-id" && property.value == "0"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-digest-count" && property.value == "1"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-digest-0-type" && property.value == "pbkdf2"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-digest-0-hash" && property.value == "sha256"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-digest-0-iterations" && property.value == "1000"
        }));
        assert!(container.properties.iter().any(|property| {
            property.key == "cryptsetup.luks-data-sector" && property.value == "4096 [bytes]"
        }));
    }

    #[test]
    fn normalizes_property_keys() {
        assert_eq!(normalize_key("key location"), "key-location");
        assert_eq!(normalize_key("PBKDF2 Hash"), "pbkdf2-hash");
    }
}
