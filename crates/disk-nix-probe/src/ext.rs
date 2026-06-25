use std::collections::BTreeMap;

use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

pub fn normalize_tune2fs(device: &str, bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let fields = parse_tune2fs(bytes)?;
    let mut graph = StorageGraph::empty();

    let filesystem_type = ext_type(&fields);
    let filesystem_id = format!("fs:{device}");
    let mut filesystem = Node::new(filesystem_id.clone(), NodeKind::Filesystem, filesystem_type)
        .with_path(device.to_string())
        .with_property("filesystem.type", filesystem_type.to_string());

    let identity = Identity {
        uuid: fields.get("Filesystem UUID").cloned(),
        partuuid: None,
        label: fields
            .get("Filesystem volume name")
            .filter(|value| value.as_str() != "<none>")
            .cloned(),
        serial: None,
        wwn: None,
    };
    if !identity.is_empty() {
        filesystem = filesystem.with_identity(identity);
    }

    if let Some(size_bytes) = size_bytes(&fields) {
        filesystem = filesystem.with_size_bytes(size_bytes);
    }

    let usage = usage(&fields);
    if !usage.is_empty() {
        filesystem = filesystem.with_usage(usage);
    }

    for &(field, property) in PROPERTIES {
        if let Some(value) = fields.get(field) {
            filesystem = filesystem.with_property(property, value.clone());
        }
    }

    graph.add_node(filesystem);
    graph.add_node(
        Node::new(
            format!("block:{device}"),
            NodeKind::DeviceMapper,
            device.to_string(),
        )
        .with_path(device.to_string()),
    );
    graph.add_edge(Edge::new(
        format!("block:{device}"),
        filesystem_id,
        Relationship::Backs,
    ));

    Ok(graph)
}

const PROPERTIES: &[(&str, &str)] = &[
    ("Filesystem state", "ext.state"),
    ("Filesystem magic number", "ext.magic-number"),
    ("Filesystem revision #", "ext.revision"),
    ("Errors behavior", "ext.errors-behavior"),
    ("FS Error count", "ext.fs-error-count"),
    ("Filesystem OS type", "ext.os-type"),
    ("Inode count", "ext.inode-count"),
    ("Free inodes", "ext.free-inodes"),
    ("Block count", "ext.block-count"),
    ("Reserved block count", "ext.reserved-block-count"),
    ("Overhead clusters", "ext.overhead-clusters"),
    ("Free blocks", "ext.free-blocks"),
    ("First block", "ext.first-block"),
    ("Block size", "ext.block-size"),
    ("Fragment size", "ext.fragment-size"),
    ("Blocks per group", "ext.blocks-per-group"),
    ("Fragments per group", "ext.fragments-per-group"),
    ("Inodes per group", "ext.inodes-per-group"),
    ("RAID stride", "ext.raid-stride"),
    ("RAID stripe width", "ext.raid-stripe-width"),
    ("Filesystem features", "ext.features"),
    ("Filesystem flags", "ext.flags"),
    ("Default directory hash", "ext.default-directory-hash"),
    ("Directory Hash Seed", "ext.directory-hash-seed"),
    ("Default mount options", "ext.default-mount-options"),
    ("Filesystem created", "ext.created"),
    ("Last mount time", "ext.last-mount-time"),
    ("Last write time", "ext.last-write-time"),
    ("Mount count", "ext.mount-count"),
    ("Maximum mount count", "ext.maximum-mount-count"),
    ("Last checked", "ext.last-checked"),
    ("Check interval", "ext.check-interval"),
    ("Lifetime writes", "ext.lifetime-writes"),
    ("Reserved blocks uid", "ext.reserved-blocks-uid"),
    ("Reserved blocks gid", "ext.reserved-blocks-gid"),
    ("First inode", "ext.first-inode"),
    ("Inode size", "ext.inode-size"),
    ("Journal inode", "ext.journal-inode"),
    ("Journal UUID", "ext.journal-uuid"),
    ("Journal backup", "ext.journal-backup"),
    ("Journal features", "ext.journal-features"),
    ("Total journal size", "ext.journal-size"),
    ("First error time", "ext.first-error-time"),
    ("First error function", "ext.first-error-function"),
    ("First error line #", "ext.first-error-line"),
    ("First error inode #", "ext.first-error-inode"),
    ("First error block #", "ext.first-error-block"),
    ("Last error time", "ext.last-error-time"),
    ("Last error function", "ext.last-error-function"),
    ("Last error line #", "ext.last-error-line"),
    ("Last error inode #", "ext.last-error-inode"),
    ("Last error block #", "ext.last-error-block"),
    ("Checksum type", "ext.checksum-type"),
    ("Checksum", "ext.checksum"),
];

fn parse_tune2fs(bytes: &[u8]) -> Result<BTreeMap<String, String>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read tune2fs output: {error}")))?;
    let mut fields = BTreeMap::new();

    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if !key.is_empty() && !value.is_empty() {
            fields.insert(key.to_string(), value.to_string());
        }
    }

    Ok(fields)
}

fn ext_type(fields: &BTreeMap<String, String>) -> &'static str {
    if fields.get("Filesystem features").is_some_and(|features| {
        features
            .split_whitespace()
            .any(|feature| feature == "extent")
    }) {
        "ext4"
    } else if fields.get("Filesystem features").is_some_and(|features| {
        features
            .split_whitespace()
            .any(|feature| feature == "has_journal")
    }) {
        "ext3"
    } else {
        "ext2"
    }
}

fn size_bytes(fields: &BTreeMap<String, String>) -> Option<u64> {
    let blocks = parse_u64(fields.get("Block count")?)?;
    let block_size = parse_u64(fields.get("Block size")?)?;
    Some(blocks.saturating_mul(block_size))
}

fn usage(fields: &BTreeMap<String, String>) -> Usage {
    let Some(block_count) = fields.get("Block count").and_then(|value| parse_u64(value)) else {
        return Usage::empty();
    };
    let Some(free_blocks) = fields.get("Free blocks").and_then(|value| parse_u64(value)) else {
        return Usage::empty();
    };
    let Some(block_size) = fields.get("Block size").and_then(|value| parse_u64(value)) else {
        return Usage::empty();
    };

    Usage {
        used_bytes: Some(
            block_count
                .saturating_sub(free_blocks)
                .saturating_mul(block_size),
        ),
        free_bytes: Some(free_blocks.saturating_mul(block_size)),
        allocated_bytes: Some(block_count.saturating_mul(block_size)),
    }
}

fn parse_u64(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .next()
        .and_then(|number| number.parse().ok())
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    const TUNE2FS: &[u8] = br#"
Filesystem volume name:   root
Filesystem UUID:          59b8deb7-5fa0-4eb3-b68c-40ac18d4f648
Filesystem magic number:  0xEF53
Filesystem revision #:    1 (dynamic)
Filesystem features:      has_journal ext_attr resize_inode dir_index filetype extent 64bit flex_bg sparse_super large_file huge_file dir_nlink extra_isize metadata_csum
Filesystem flags:         signed_directory_hash
Default directory hash:   half_md4
Directory Hash Seed:      11111111-2222-3333-4444-555555555555
Default mount options:    user_xattr acl
Filesystem state:         clean
Errors behavior:          Continue
FS Error count:           2
Filesystem OS type:       Linux
Inode count:              30531584
Block count:              122096646
Reserved block count:     6104832
Overhead clusters:        123456
Free blocks:              73328197
Free inodes:              27187554
First block:              0
Block size:               4096
Fragment size:            4096
Blocks per group:         32768
Fragments per group:      32768
Inodes per group:         8192
RAID stride:              128
RAID stripe width:        256
Filesystem created:       Mon Jan 01 00:00:00 2024
Last mount time:          Mon Jun 22 12:00:00 2026
Last write time:          Mon Jun 22 12:00:00 2026
Mount count:              12
Maximum mount count:      -1
Last checked:             Mon Jan 01 00:00:00 2024
Check interval:           0 (<none>)
Lifetime writes:          189 GB
Inode size:               256
Journal inode:            8
Journal UUID:             99999999-aaaa-bbbb-cccc-dddddddddddd
Total journal size:       1024M
First error time:         Mon Jun 22 12:30:00 2026
First error function:     ext4_lookup
First error line #:       1234
First error inode #:      42
First error block #:      9001
Last error time:          Mon Jun 22 12:45:00 2026
Last error function:      ext4_journal_check_start
Last error line #:        5678
Last error inode #:       43
Last error block #:       9002
Checksum type:            crc32c
Checksum:                 0x12345678
"#;

    #[test]
    fn normalizes_tune2fs_superblock_metadata() {
        let graph = normalize_tune2fs("/dev/sda2", TUNE2FS).expect("fixture should parse");
        let filesystem = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "fs:/dev/sda2")
            .expect("filesystem exists");

        assert_eq!(filesystem.kind, NodeKind::Filesystem);
        assert_eq!(filesystem.path.as_deref(), Some("/dev/sda2"));
        assert_eq!(
            filesystem.identity.uuid.as_deref(),
            Some("59b8deb7-5fa0-4eb3-b68c-40ac18d4f648")
        );
        assert_eq!(filesystem.identity.label.as_deref(), Some("root"));
        assert_eq!(filesystem.size_bytes, Some(500_107_862_016));
        assert_eq!(
            filesystem.usage.as_ref().and_then(|usage| usage.free_bytes),
            Some(300_352_294_912)
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.features" && property.value.contains("metadata_csum")
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.lifetime-writes" && property.value == "189 GB"
        }));
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ext.magic-number" && property.value == "0xEF53"
            })
        );
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ext.revision" && property.value == "1 (dynamic)"
            })
        );
        assert!(
            filesystem
                .properties
                .iter()
                .any(|property| { property.key == "ext.fs-error-count" && property.value == "2" })
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.first-error-function" && property.value == "ext4_lookup"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.first-error-block" && property.value == "9001"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.last-error-function"
                && property.value == "ext4_journal_check_start"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.last-error-block" && property.value == "9002"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.overhead-clusters" && property.value == "123456"
        }));
        assert!(
            filesystem
                .properties
                .iter()
                .any(|property| { property.key == "ext.first-block" && property.value == "0" })
        );
        assert!(
            filesystem
                .properties
                .iter()
                .any(|property| { property.key == "ext.raid-stride" && property.value == "128" })
        );
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.raid-stripe-width" && property.value == "256"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.default-directory-hash" && property.value == "half_md4"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.directory-hash-seed"
                && property.value == "11111111-2222-3333-4444-555555555555"
        }));
        assert!(filesystem.properties.iter().any(|property| {
            property.key == "ext.journal-uuid"
                && property.value == "99999999-aaaa-bbbb-cccc-dddddddddddd"
        }));
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ext.checksum-type" && property.value == "crc32c"
            })
        );
        assert!(
            filesystem.properties.iter().any(|property| {
                property.key == "ext.checksum" && property.value == "0x12345678"
            })
        );
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sda2"
                && edge.to.0 == "fs:/dev/sda2"
                && edge.relationship == Relationship::Backs
        }));
    }
}
