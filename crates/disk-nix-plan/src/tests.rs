use disk_nix_model::{Identity, Usage};
use serde::Deserialize;

use super::*;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MigrationExampleFixture {
    name: String,
    base_example: String,
    description: String,
    target_spec: serde_json::Value,
    current_graph: StorageGraph,
    expected_remaining_action_ids: Vec<String>,
    expected_suppressed_action_ids: Vec<String>,
}

include!("tests/part_01.rs");
include!("tests/part_02.rs");
include!("tests/part_03.rs");
include!("tests/part_04.rs");
include!("tests/part_05.rs");
include!("tests/part_06.rs");
include!("tests/part_07.rs");
include!("tests/part_08.rs");
include!("tests/part_09.rs");
include!("tests/part_10.rs");
include!("tests/part_11.rs");
include!("tests/part_12.rs");
include!("tests/part_13.rs");
include!("tests/part_14.rs");
include!("tests/part_15.rs");
include!("tests/part_16.rs");
