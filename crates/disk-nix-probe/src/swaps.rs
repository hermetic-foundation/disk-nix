use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph, Usage};

use crate::ProbeError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SwapEntry {
    filename: String,
    swap_type: String,
    size_kib: u64,
    used_kib: u64,
    priority: i64,
}

pub fn normalize_proc_swaps(bytes: &[u8]) -> Result<StorageGraph, ProbeError> {
    let entries = parse_proc_swaps(bytes)?;
    let mut graph = StorageGraph::empty();

    for entry in entries {
        add_swap(&mut graph, entry);
    }

    Ok(graph)
}

fn parse_proc_swaps(bytes: &[u8]) -> Result<Vec<SwapEntry>, ProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| ProbeError::Adapter(format!("failed to read /proc/swaps: {error}")))?;
    let mut entries = Vec::new();

    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .skip(1)
    {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            return Err(ProbeError::Adapter(format!(
                "/proc/swaps row has {} fields, expected 5",
                fields.len()
            )));
        }

        entries.push(SwapEntry {
            filename: fields[0].to_string(),
            swap_type: fields[1].to_string(),
            size_kib: parse_number(fields[2], "swap size")?,
            used_kib: parse_number(fields[3], "swap used")?,
            priority: parse_number(fields[4], "swap priority")?,
        });
    }

    Ok(entries)
}

fn add_swap(graph: &mut StorageGraph, entry: SwapEntry) {
    let block_id = format!("block:{}", entry.filename);
    graph.add_node(
        Node::new(block_id.clone(), NodeKind::Swap, entry.filename.clone())
            .with_path(entry.filename.clone())
            .with_size_bytes(kib_to_bytes(entry.size_kib))
            .with_usage(Usage {
                used_bytes: Some(kib_to_bytes(entry.used_kib)),
                free_bytes: Some(kib_to_bytes(entry.size_kib.saturating_sub(entry.used_kib))),
                allocated_bytes: Some(kib_to_bytes(entry.size_kib)),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", entry.swap_type.clone())
            .with_property("swap.priority", entry.priority.to_string()),
    );

    let active_id = format!("swap:{}", entry.filename);
    graph.add_node(
        Node::new(active_id.clone(), NodeKind::Swap, entry.filename.clone())
            .with_path(entry.filename)
            .with_size_bytes(kib_to_bytes(entry.size_kib))
            .with_usage(Usage {
                used_bytes: Some(kib_to_bytes(entry.used_kib)),
                free_bytes: Some(kib_to_bytes(entry.size_kib.saturating_sub(entry.used_kib))),
                allocated_bytes: Some(kib_to_bytes(entry.size_kib)),
            })
            .with_property("swap.active", "true")
            .with_property("swap.type", entry.swap_type)
            .with_property("swap.priority", entry.priority.to_string()),
    );
    graph.add_edge(Edge::new(block_id, active_id, Relationship::Backs));
}

fn parse_number<T>(value: &str, field: &str) -> Result<T, ProbeError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error| ProbeError::Adapter(format!("failed to parse {field}: {error}")))
}

fn kib_to_bytes(value: u64) -> u64 {
    value.saturating_mul(1024)
}

#[cfg(test)]
mod tests {
    use disk_nix_model::Relationship;

    use super::*;

    const PROC_SWAPS: &[u8] = br#"
Filename				Type		Size		Used		Priority
/dev/sda3                               partition	9227496		52336		-2
/swapfile                               file		1048576		0		10
"#;

    #[test]
    fn normalizes_active_swaps() {
        let graph = normalize_proc_swaps(PROC_SWAPS).expect("fixture should parse");
        let swap = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "swap:/dev/sda3")
            .expect("active swap node exists");

        assert_eq!(swap.size_bytes, Some(9_448_955_904));
        assert_eq!(
            swap.usage.as_ref().and_then(|usage| usage.used_bytes),
            Some(53_592_064)
        );
        assert!(swap
            .properties
            .iter()
            .any(|property| { property.key == "swap.priority" && property.value == "-2" }));
        assert!(graph.edges.iter().any(|edge| {
            edge.from.0 == "block:/dev/sda3"
                && edge.to.0 == "swap:/dev/sda3"
                && edge.relationship == Relationship::Backs
        }));
    }
}
