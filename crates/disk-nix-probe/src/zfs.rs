use std::collections::BTreeMap;

use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

include!("zfs/types.rs");
include!("zfs/parsing.rs");
include!("zfs/graph.rs");
include!("zfs/helpers.rs");

pub fn normalize_zfs(
    zpool_list: &[u8],
    zpool_get: &[u8],
    zfs_list: &[u8],
    zfs_holds: &[u8],
    zpool_status: &[u8],
) -> Result<StorageGraph, ProbeError> {
    let mut graph = StorageGraph::empty();

    for pool in parse_zpools(zpool_list)? {
        add_pool(&mut graph, pool);
    }
    for property in parse_zpool_properties(zpool_get)? {
        add_pool_property(&mut graph, property);
    }
    let datasets = parse_datasets(zfs_list)?;
    let dataset_kinds = dataset_kinds(&datasets);
    for dataset in datasets {
        add_dataset(&mut graph, dataset, &dataset_kinds);
    }
    for hold in parse_zfs_holds(zfs_holds)? {
        add_snapshot_hold(&mut graph, hold);
    }
    for pool in parse_zpool_status(zpool_status)? {
        add_status_pool(&mut graph, pool);
    }

    Ok(graph)
}

include!("zfs/tests.rs");
