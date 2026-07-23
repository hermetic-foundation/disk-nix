use disk_nix_model::{Edge, Identity, Node, NodeKind, Relationship, StorageGraph, Usage};
use serde::Deserialize;
use serde_json::Value;

use crate::ProbeError;

include!("nvme/types.rs");
include!("nvme/list_subsystems.rs");
include!("nvme/id_namespace.rs");
include!("nvme/id_controller.rs");
include!("nvme/smart_log.rs");
include!("nvme/graph.rs");
include!("nvme/helpers.rs");
include!("nvme/tests.rs");
