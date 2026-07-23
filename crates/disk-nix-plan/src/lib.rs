use disk_nix_model::{Node, NodeKind, Relationship, StorageGraph};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

include!("sections/model.rs");
include!("sections/dependencies.rs");
include!("sections/topology_properties.rs");
include!("sections/local_diagnostics.rs");
include!("sections/mapping_diagnostics.rs");
include!("sections/storage_diagnostics.rs");
include!("sections/action_fields.rs");
include!("sections/action_builders.rs");
include!("sections/operation_classification.rs");
include!("sections/capabilities.rs");

#[cfg(test)]
mod tests;
