#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const ZPOOL: &[u8] = b"tank\t1000\t400\t600\tONLINE\t40%\t1.00x\t12%\t/mnt/rescue\n";
    const ZPOOL_GET: &[u8] = b"tank\taltroot\t/mnt/rescue\n\
tank\tashift\t12\n\
tank\tautotrim\ton\n\
tank\tautoexpand\toff\n\
tank\tautoreplace\toff\n\
tank\tbootfs\ttank/root\n\
tank\tcachefile\t/etc/zfs/zpool.cache\n\
tank\tcomment\tprimary pool\n\
tank\tdelegation\ton\n\
tank\tfailmode\twait\n\
tank\tlistsnapshots\toff\n\
tank\tmultihost\toff\n";
    const ZFS: &[u8] = b"tank\tfilesystem\t100\t900\t100\t/tank\t-\t-\tlz4\tnone\tnone\toff\t-\t-\t131072\toff\ton\t1\tstandard\tall\tall\ton\toff\thidden\toff\tsa\n\
tank/home\tfilesystem\t200\t800\t200\t/home\t-\t-\tzstd\t1073741824\t268435456\taes-256-gcm\tavailable\t-\t1048576\toff\tsha512\t2\tdisabled\tmetadata\tall\toff\ton\tvisible\tposixacl\tsa\n\
tank/home@daily\tsnapshot\t10\t-\t10\t-\t-\t2\tzstd\t-\t-\taes-256-gcm\tavailable\t-\t1048576\toff\tsha512\t2\tdisabled\tmetadata\tall\toff\ton\tvisible\tposixacl\tsa\n\
tank/vm\tvolume\t50\t950\t50\t-\t-\t-\tlz4\t-\t-\toff\t-\t85899345920\t-\ton\tfletcher4\t1\tstandard\tall\tnone\toff\toff\thidden\toff\ton\n\
tank/vm@clean\tsnapshot\t5\t-\t5\t-\t-\t1\tlz4\t-\t-\toff\t-\t-\t-\ton\tfletcher4\t1\tstandard\tall\tnone\toff\toff\thidden\toff\ton\n";
    const ZFS_HOLDS: &[u8] = b"tank/home@daily\tdisk-nix-retain\tWed Jun 24 18:00 2026\n\
tank/home@daily\tbackup-job\tWed Jun 24 18:01 2026\n";
    const ZPOOL_STATUS: &[u8] = br#"
  pool: tank
 state: ONLINE
  scan: scrub repaired 0B in 00:01:02 with 0 errors on Sun Jun 21 00:00:00 2026
config:

        NAME                                      STATE     READ WRITE CKSUM
        tank                                      ONLINE       0     0     0
          mirror-0                                ONLINE       0     0     0
            /dev/disk/by-id/disk-a-part1         ONLINE       0     0     0
            /dev/disk/by-id/disk-b-part1         ONLINE       0     0     0
        logs
          /dev/disk/by-id/log0                   ONLINE       0     0     0
        cache
          /dev/disk/by-id/cache0                 ONLINE       0     0     0

errors: No known data errors
"#;
    const DEGRADED_ZPOOL_STATUS: &[u8] = br#"
  pool: tank
 state: DEGRADED
status: One or more devices could not be used because the label is missing or invalid.
action: Replace the device using 'zpool replace'.
  scan: resilvered 1024B in 00:00:01 with 0 errors on Sun Jun 21 00:00:00 2026
config:

        NAME                                      STATE     READ WRITE CKSUM
        tank                                      DEGRADED     4     5     6
          /dev/disk/by-id/disk-a-part1           ONLINE       0     0     0

errors: No known data errors
"#;

    #[test]
    fn normalizes_zfs_pool_datasets_snapshots_and_zvols() {
        let graph = normalize_zfs(ZPOOL, ZPOOL_GET, ZFS, ZFS_HOLDS, ZPOOL_STATUS)
            .expect("fixture should parse");

        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
        );
        let pool = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
            .expect("pool node exists");
        assert!(
            pool.properties
                .iter()
                .any(|property| { property.key == "zfs.pool-capacity" && property.value == "40%" })
        );
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-dedupratio" && property.value == "1.00x"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-fragmentation" && property.value == "12%"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-altroot" && property.value == "/mnt/rescue"
        }));
        assert!(
            pool.properties
                .iter()
                .any(|property| property.key == "zfs.pool-ashift" && property.value == "12")
        );
        assert!(
            pool.properties
                .iter()
                .any(|property| property.key == "zfs.pool-autotrim" && property.value == "on")
        );
        assert!(
            pool.properties.iter().any(|property| {
                property.key == "zfs.pool-autoexpand" && property.value == "off"
            })
        );
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-cachefile" && property.value == "/etc/zfs/zpool.cache"
        }));
        assert!(
            pool.properties.iter().any(|property| {
                property.key == "zfs.pool-failmode" && property.value == "wait"
            })
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsDataset && node.name == "tank/home")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/vm@clean")
        );
        let snapshot = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsSnapshot && node.name == "tank/home@daily")
            .expect("snapshot node exists");
        assert!(
            snapshot
                .properties
                .iter()
                .any(|property| property.key == "zfs.userrefs" && property.value == "2")
        );
        assert!(snapshot.properties.iter().any(|property| {
            property.key == "zfs.holds" && property.value == "disk-nix-retain"
        }));
        assert!(snapshot.properties.iter().any(|property| {
            property.key == "zfs.hold.disk-nix-retain" && property.value == "Wed Jun 24 18:00 2026"
        }));
        assert!(
            snapshot
                .properties
                .iter()
                .any(|property| property.key == "zfs.hold.backup-job")
        );
        let dataset = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsDataset && node.name == "tank/home")
            .expect("dataset node exists");
        assert!(
            dataset
                .properties
                .iter()
                .any(|property| property.key == "zfs.compression" && property.value == "zstd")
        );
        assert!(dataset.properties.iter().any(|property| {
            property.key == "zfs.encryption" && property.value == "aes-256-gcm"
        }));
        assert!(
            dataset.properties.iter().any(|property| {
                property.key == "zfs.recordsize" && property.value == "1048576"
            })
        );
        assert!(
            dataset
                .properties
                .iter()
                .any(|property| property.key == "zfs.checksum" && property.value == "sha512")
        );
        assert!(
            dataset
                .properties
                .iter()
                .any(|property| property.key == "zfs.primarycache" && property.value == "metadata")
        );
        let zvol = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Zvol && node.name == "tank/vm")
            .expect("zvol node exists");
        assert!(
            zvol.properties
                .iter()
                .any(|property| property.key == "zfs.volsize" && property.value == "85899345920")
        );
        assert!(
            graph
                .nodes
                .iter()
                .any(|node| node.kind == NodeKind::Zvol && node.name == "tank/vm")
        );
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.relationship == Relationship::MountedAt)
        );
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::ZfsVdev
                && node.name == "mirror-0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "zfs.vdev-role" && property.value == "data")
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::ZfsVdev
                && node.name == "/dev/disk/by-id/cache0"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "zfs.vdev-role" && property.value == "cache")
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/disk/by-id/disk-a-part1"
                && edge.to.0 == "zfs-vdev:tank:/dev/disk/by-id/disk-a-part1"
                && edge.relationship == Relationship::Backs
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "zfs-snapshot:tank/home@daily"
                && edge.to.0 == "zfs-dataset:tank/home"
                && edge.relationship == Relationship::SnapshotOf
        }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "zfs-snapshot:tank/vm@clean"
                && edge.to.0 == "zvol:tank/vm"
                && edge.relationship == Relationship::SnapshotOf
        }));
    }

    #[test]
    fn normalizes_zpool_status_advisory_fields() {
        let graph = normalize_zfs(ZPOOL, ZPOOL_GET, ZFS, ZFS_HOLDS, DEGRADED_ZPOOL_STATUS)
            .expect("fixture should parse");
        let pool = graph
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::ZfsPool && node.name == "tank")
            .expect("pool node exists");

        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.status"
                && property.value
                    == "One or more devices could not be used because the label is missing or invalid."
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.action"
                && property.value == "Replace the device using 'zpool replace'."
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.scan"
                && property.value
                    == "resilvered 1024B in 00:00:01 with 0 errors on Sun Jun 21 00:00:00 2026"
        }));
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.errors" && property.value == "No known data errors"
        }));
        assert!(
            pool.properties.iter().any(|property| {
                property.key == "zfs.pool-read-errors" && property.value == "4"
            })
        );
        assert!(
            pool.properties.iter().any(|property| {
                property.key == "zfs.pool-write-errors" && property.value == "5"
            })
        );
        assert!(pool.properties.iter().any(|property| {
            property.key == "zfs.pool-checksum-errors" && property.value == "6"
        }));
    }
}
