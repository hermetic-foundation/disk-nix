#[derive(Debug, Clone)]
struct SolveDisk {
    key: String,
    path: String,
    size_gib: u64,
    media: String,
    tier: String,
    primary_boot: bool,
    slice_count: u64,
}

#[derive(Debug, Clone)]
struct SolvePool {
    key: String,
    zfs: Map<String, Value>,
    disk_indexes: Vec<usize>,
    solution: VdevSolution,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnassignedSlicePolicy {
    Allow,
    Forbid,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VdevShape {
    kind: String,
    width: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VdevChoice {
    shape_index: usize,
    disk_indexes: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VdevSolution {
    used_slices: u64,
    preferred_shape_counts: Vec<u64>,
    choices: Vec<VdevChoice>,
}

pub fn lower_solved_document_from_json_bytes(bytes: &[u8]) -> Result<Value, PlanDocumentError> {
    let value: Value = serde_json::from_slice(bytes)?;
    validate_spec_version(&value)?;
    lower_solved_document(&value)
}

pub fn lower_solved_document(value: &Value) -> Result<Value, PlanDocumentError> {
    let mut document = value.clone();
    let spec = document
        .get("spec")
        .cloned()
        .unwrap_or_else(|| document.clone());
    let lowered = lower_solved_layouts(&spec)?.unwrap_or(spec);
    if document.get("spec").is_some() {
        let Some(object) = document.as_object_mut() else {
            return Err(solver_error("top-level disk-nix document must be a JSON object"));
        };
        object.insert("spec".to_string(), lowered);
        Ok(Value::Object(object.clone()))
    } else {
        Ok(lowered)
    }
}

fn lower_solved_layouts(spec: &Value) -> Result<Option<Value>, PlanDocumentError> {
    let Some(layouts) = spec
        .get("solve")
        .and_then(|solve| solve.get("layouts"))
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };
    if layouts.is_empty() {
        return Ok(None);
    }

    let mut generated = Map::new();
    for (layout_name, layout) in layouts {
        lower_layout(layout_name, layout, &mut generated)?;
    }

    let mut lowered = generated;
    if let Some(existing) = spec.as_object() {
        merge_spec_objects(&mut lowered, existing);
    }
    Ok(Some(Value::Object(lowered)))
}

fn lower_layout(
    layout_name: &str,
    layout: &Value,
    generated: &mut Map<String, Value>,
) -> Result<(), PlanDocumentError> {
    let Some(disks_value) = layout.get("disks").and_then(Value::as_object) else {
        return Err(solver_error(format!(
            "solve layout {layout_name} must declare disks"
        )));
    };
    let Some(zfs) = layout.get("zfs").and_then(Value::as_object) else {
        return Err(solver_error(format!(
            "solve layout {layout_name} must declare zfs"
        )));
    };

    let boot = layout.get("boot").and_then(Value::as_object);
    let swap = layout.get("swap").and_then(Value::as_object);
    let boot_size = boot
        .and_then(|boot| boot.get("size"))
        .and_then(Value::as_str)
        .and_then(parse_size_gib)
        .unwrap_or(1);
    let slice_size = zfs
        .get("sliceSize")
        .and_then(Value::as_str)
        .and_then(parse_size_gib)
        .unwrap_or(100);
    if slice_size == 0 {
        return Err(solver_error(format!(
            "solve layout {layout_name} zfs.sliceSize must be greater than zero"
        )));
    }

    let primary_boot_disk = boot
        .and_then(|boot| boot.get("primaryDisk"))
        .and_then(Value::as_str);
    let boot_mountpoint = boot
        .and_then(|boot| boot.get("mountpoint"))
        .and_then(Value::as_str)
        .unwrap_or("/boot");
    let replicated_boot = boot
        .and_then(|boot| boot.get("type"))
        .and_then(Value::as_str)
        == Some("efi-replicated");
    let tail_swap = swap
        .and_then(|swap| swap.get("type"))
        .and_then(Value::as_str)
        == Some("tail");
    let mut disks = disks_value
        .iter()
        .map(|(disk_key, disk)| {
            let Some(path) = disk.get("path").and_then(Value::as_str) else {
                return Err(solver_error(format!(
                    "solve layout {layout_name} disk {disk_key} must declare path"
                )));
            };
            let Some(size_gib) = disk
                .get("size")
                .and_then(Value::as_str)
                .and_then(parse_size_gib)
            else {
                return Err(solver_error(format!(
                    "solve layout {layout_name} disk {disk_key} must declare a parseable size"
                )));
            };
            let media = disk
                .get("media")
                .or_else(|| disk.get("transport"))
                .and_then(Value::as_str)
                .unwrap_or("disk")
                .to_string();
            let tier = disk
                .get("tier")
                .or_else(|| disk.get("poolTier"))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| infer_disk_tier(disk, path, &media));
            let primary_boot = primary_boot_disk.is_some_and(|primary| primary == disk_key)
                || disk
                    .get("primaryBoot")
                    .and_then(Value::as_bool)
                .unwrap_or(false);
            let usable_gib = size_gib.saturating_sub(boot_size);
            Ok(SolveDisk {
                key: disk_key.clone(),
                path: path.to_string(),
                size_gib,
                media,
                tier,
                primary_boot,
                slice_count: usable_gib / slice_size,
            })
        })
        .collect::<Result<Vec<_>, PlanDocumentError>>()?;
    disks.sort_by(|left, right| {
        left.primary_boot
            .cmp(&right.primary_boot)
            .reverse()
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.key.cmp(&right.key))
    });

    if disks.is_empty() {
        return Err(solver_error(format!(
            "solve layout {layout_name} must include at least one usable disk"
        )));
    }

    let pools = solve_pools(layout_name, zfs, &disks)?;

    insert_generated_disks(generated, &disks);
    insert_generated_partitions(
        generated,
        layout_name,
        &disks,
        &pools,
        boot_size,
        slice_size,
        replicated_boot,
        tail_swap,
    );
    insert_generated_boot_filesystems(
        generated,
        layout_name,
        &disks,
        replicated_boot,
        boot_mountpoint,
    );
    insert_generated_swaps(generated, layout_name, &disks, &pools, swap, tail_swap);
    for pool in pools {
        let pool_shapes = parse_vdev_shapes(layout_name, &pool.zfs)?;
        let selected_disks = pool
            .disk_indexes
            .iter()
            .map(|index| disks[*index].clone())
            .collect::<Vec<_>>();
        insert_generated_pool(
            generated,
            &pool.zfs,
            &selected_disks,
            &pool_shapes,
            pool.solution,
        );
        insert_generated_datasets(generated, &pool.zfs);
    }
    Ok(())
}

fn insert_generated_disks(generated: &mut Map<String, Value>, disks: &[SolveDisk]) {
    let disks_collection = collection_mut(generated, "disks");
    for disk in disks {
        let mut object = Map::new();
        object.insert("operation".to_string(), Value::String("create".to_string()));
        object.insert("target".to_string(), Value::String(disk.path.clone()));
        object.insert("partitionType".to_string(), Value::String("gpt".to_string()));
        object.insert("preserveData".to_string(), Value::Bool(false));
        object.insert(
            "properties".to_string(),
            object_value([
                ("media", Value::String(disk.media.clone())),
                ("tier", Value::String(disk.tier.clone())),
                ("size", Value::String(format!("{}GiB", disk.size_gib))),
                ("byId", Value::String(disk.path.clone())),
            ]),
        );
        disks_collection.insert(disk.key.clone(), Value::Object(object));
    }
}

fn insert_generated_partitions(
    generated: &mut Map<String, Value>,
    layout_name: &str,
    disks: &[SolveDisk],
    pools: &[SolvePool],
    boot_size: u64,
    slice_size: u64,
    replicated_boot: bool,
    tail_swap: bool,
) {
    let partitions = collection_mut(generated, "partitions");
    for disk in disks {
        let boot_partition_number = 1;
        if replicated_boot {
            partitions.insert(
                format!("{layout_name}-{}-efi", disk.key),
                partition_object(
                    &disk.path,
                    boot_partition_number,
                    "1MiB",
                    &format!("{}MiB", boot_size * 1024 + 1),
                    &format!("{}-part{boot_partition_number}", disk.path),
                    "EF00",
                    Some(&format!("BOOT-{}", disk.key.to_uppercase())),
                ),
            );
        }

        let pool_key = pools
            .iter()
            .find(|pool| pool.disk_indexes.iter().any(|index| disks[*index].key == disk.key))
            .map(|pool| pool.key.as_str());
        let zfs_slice_count = pool_key.map_or(0, |_| disk.slice_count);
        let zfs_partition_prefix = pool_key
            .filter(|_| pools.len() > 1)
            .map_or_else(
                || format!("{layout_name}-{}-zfs", disk.key),
                |pool_key| format!("{layout_name}-{}-{pool_key}-zfs", disk.key),
            );
        for slice_index in 0..zfs_slice_count {
            let partition_number = slice_index + 2;
            let end_gib = boot_size + (slice_index + 1) * slice_size;
            partitions.insert(
                format!("{zfs_partition_prefix}-{}", slice_index + 1),
                partition_object(
                    &disk.path,
                    partition_number,
                    &zfs_slice_start(boot_size, slice_size, slice_index),
                    &format!("{end_gib}GiB"),
                    &format!("{}-part{partition_number}", disk.path),
                    "BF01",
                    None,
                ),
            );
        }

        if tail_swap {
            let partition_number = zfs_slice_count + 2;
            let start_gib = boot_size + zfs_slice_count * slice_size;
            partitions.insert(
                format!("{layout_name}-{}-swap", disk.key),
                partition_object(
                    &disk.path,
                    partition_number,
                    &format!("{start_gib}GiB"),
                    "100%",
                    &format!("{}-part{partition_number}", disk.path),
                    "8200",
                    Some(&format!("swap-{}", disk.key)),
                ),
            );
        }
    }
}

fn zfs_slice_start(boot_size: u64, slice_size: u64, slice_index: u64) -> String {
    if slice_index == 0 {
        format!("{}MiB", boot_size * 1024 + 1)
    } else {
        format!("{}GiB", boot_size + slice_index * slice_size)
    }
}

fn insert_generated_boot_filesystems(
    generated: &mut Map<String, Value>,
    layout_name: &str,
    disks: &[SolveDisk],
    replicated_boot: bool,
    boot_mountpoint: &str,
) {
    if !replicated_boot {
        return;
    }

    let filesystems = collection_mut(generated, "filesystems");
    for disk in disks {
        let mut object = Map::new();
        object.insert("operation".to_string(), Value::String("format".to_string()));
        object.insert(
            "device".to_string(),
            Value::String(format!("{}-part1", disk.path)),
        );
        object.insert("fsType".to_string(), Value::String("vfat".to_string()));
        object.insert("preserveData".to_string(), Value::Bool(false));
        object.insert(
            "properties".to_string(),
            object_value([(
                "label",
                Value::String(if disk.primary_boot {
                    "BOOT".to_string()
                } else {
                    format!("BOOT-{}", disk.key.to_uppercase())
                }),
            )]),
        );
        if disk.primary_boot {
            object.insert(
                "mountpoint".to_string(),
                Value::String(boot_mountpoint.to_string()),
            );
            object.insert("neededForBoot".to_string(), Value::Bool(true));
            object.insert(
                "options".to_string(),
                Value::Array(vec![
                    Value::String("fmask=0077".to_string()),
                    Value::String("dmask=0077".to_string()),
                ]),
            );
        }
        filesystems.insert(format!("{layout_name}-boot-{}", disk.key), Value::Object(object));
    }
}

fn insert_generated_swaps(
    generated: &mut Map<String, Value>,
    layout_name: &str,
    disks: &[SolveDisk],
    pools: &[SolvePool],
    swap: Option<&Map<String, Value>>,
    tail_swap: bool,
) {
    if !tail_swap {
        return;
    }

    let swaps = collection_mut(generated, "swaps");
    let priorities = swap
        .and_then(|swap| swap.get("priorities"))
        .and_then(Value::as_object);
    for disk in disks {
        let partition_number = zfs_slice_count_for_disk(disks, pools, disk) + 2;
        let priority = priorities
            .and_then(|priorities| priorities.get(&disk.key))
            .or_else(|| priorities.and_then(|priorities| priorities.get(&disk.media)))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let mut object = Map::new();
        object.insert("operation".to_string(), Value::String("format".to_string()));
        object.insert(
            "device".to_string(),
            Value::String(format!("{}-part{partition_number}", disk.path)),
        );
        object.insert(
            "priority".to_string(),
            Value::Number(serde_json::Number::from(priority)),
        );
        object.insert("preserveData".to_string(), Value::Bool(false));
        object.insert(
            "properties".to_string(),
            object_value([("label", Value::String(format!("swap-{}", disk.key)))]),
        );
        swaps.insert(format!("{layout_name}-swap-{}", disk.key), Value::Object(object));
    }
}

fn zfs_slice_count_for_disk(disks: &[SolveDisk], pools: &[SolvePool], disk: &SolveDisk) -> u64 {
    pools
        .iter()
        .any(|pool| {
            pool.disk_indexes
                .iter()
                .any(|index| disks[*index].key == disk.key)
        })
        .then_some(disk.slice_count)
        .unwrap_or(0)
}

fn insert_generated_pool(
    generated: &mut Map<String, Value>,
    zfs: &Map<String, Value>,
    disks: &[SolveDisk],
    shapes: &[VdevShape],
    solution: VdevSolution,
) {
    let pool_name = zfs
        .get("pool")
        .and_then(Value::as_str)
        .unwrap_or("zroot");
    let mut next_slice_by_disk = vec![0_u64; disks.len()];
    let mut devices = Vec::new();
    for choice in solution.choices {
        let shape = &shapes[choice.shape_index];
        if !is_single_vdev_shape(&shape.kind) {
            devices.push(Value::String(shape.kind.clone()));
        }
        for disk_index in choice.disk_indexes {
            next_slice_by_disk[disk_index] += 1;
            devices.push(Value::String(format!(
                "{}-part{}",
                disks[disk_index].path,
                next_slice_by_disk[disk_index] + 1
            )));
        }
    }

    let mut object = Map::new();
    object.insert("operation".to_string(), Value::String("create".to_string()));
    object.insert("devices".to_string(), Value::Array(devices));
    if let Some(properties) = zfs.get("properties") {
        object.insert("properties".to_string(), properties.clone());
    }
    collection_mut(generated, "pools").insert(pool_name.to_string(), Value::Object(object));
}

fn validate_solution(
    layout_name: &str,
    counts: &[u64],
    solution: &VdevSolution,
    require_redundant: bool,
    unassigned_slice_policy: UnassignedSlicePolicy,
) -> Result<(), PlanDocumentError> {
    let total_slices = counts.iter().sum::<u64>();
    if require_redundant && total_slices > 0 && solution.used_slices == 0 {
        return Err(solver_error(format!(
            "solve layout {layout_name} cannot form any redundant vdev from {total_slices} slice(s)"
        )));
    }

    if unassigned_slice_policy == UnassignedSlicePolicy::Forbid
        && solution.used_slices != total_slices
    {
        return Err(solver_error(format!(
            "solve layout {layout_name} leaves {} full ZFS slice(s) unassigned; set zfs.vdevs.unassignedSlicePolicy = \"allow\" or add compatible vdev shapes",
            total_slices - solution.used_slices
        )));
    }

    Ok(())
}

fn solve_pools(
    layout_name: &str,
    zfs: &Map<String, Value>,
    disks: &[SolveDisk],
) -> Result<Vec<SolvePool>, PlanDocumentError> {
    let pool_specs = zfs
        .get("pools")
        .and_then(Value::as_object)
        .map(|pools| {
            pools
                .iter()
                .map(|(pool_key, pool_value)| {
                    let Some(pool_object) = pool_value.as_object() else {
                        return Err(solver_error(format!(
                            "solve layout {layout_name} zfs pool {pool_key} must be an object"
                        )));
                    };
                    let mut merged = zfs.clone();
                    merged.remove("pool");
                    merged.remove("pools");
                    merge_spec_objects(&mut merged, pool_object);
                    merged
                        .entry("pool".to_string())
                        .or_insert_with(|| Value::String(pool_key.clone()));
                    Ok((pool_key.clone(), merged))
                })
                .collect::<Result<Vec<_>, PlanDocumentError>>()
        })
        .transpose()?
        .unwrap_or_else(|| {
            vec![(
                zfs.get("pool")
                    .and_then(Value::as_str)
                    .unwrap_or("zroot")
                    .to_string(),
                zfs.clone(),
            )]
        });

    let mut owner_by_disk = vec![None::<String>; disks.len()];
    let mut pools = Vec::new();
    for (pool_key, pool_zfs) in pool_specs {
        let disk_indexes = select_pool_disks(layout_name, &pool_key, &pool_zfs, disks)?;
        if disk_indexes.is_empty() {
            return Err(solver_error(format!(
                "solve layout {layout_name} zfs pool {pool_key} selected no disks"
            )));
        }
        for disk_index in &disk_indexes {
            if let Some(owner) = &owner_by_disk[*disk_index] {
                return Err(solver_error(format!(
                    "solve layout {layout_name} disk {} is selected by both zfs pools {owner} and {pool_key}",
                    disks[*disk_index].key
                )));
            }
            owner_by_disk[*disk_index] = Some(pool_key.clone());
        }

        let shapes = parse_vdev_shapes(layout_name, &pool_zfs)?;
        let counts = disk_indexes
            .iter()
            .map(|disk_index| disks[*disk_index].slice_count)
            .collect::<Vec<_>>();
        let solution = solve_vdevs(&counts, &shapes).unwrap_or(VdevSolution {
            used_slices: 0,
            preferred_shape_counts: vec![0; shapes.len()],
            choices: Vec::new(),
        });
        validate_solution(
            layout_name,
            &counts,
            &solution,
            pool_zfs
                .get("vdevs")
                .and_then(|vdevs| vdevs.get("requireRedundant"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
            pool_zfs
                .get("vdevs")
                .and_then(|vdevs| vdevs.get("unassignedSlicePolicy"))
                .or_else(|| pool_zfs.get("unassignedSlicePolicy"))
                .and_then(Value::as_str)
                .map(parse_unassigned_slice_policy)
                .transpose()?
                .unwrap_or(UnassignedSlicePolicy::Allow),
        )?;
        pools.push(SolvePool {
            key: pool_key,
            zfs: pool_zfs,
            disk_indexes,
            solution,
        });
    }
    Ok(pools)
}

fn select_pool_disks(
    layout_name: &str,
    pool_key: &str,
    zfs: &Map<String, Value>,
    disks: &[SolveDisk],
) -> Result<Vec<usize>, PlanDocumentError> {
    if let Some(disks_value) = zfs.get("disks") {
        let Some(selected) = disks_value.as_array() else {
            return Err(solver_error(format!(
                "solve layout {layout_name} zfs pool {pool_key} disks must be an array"
            )));
        };
        return selected
            .iter()
            .map(|disk| {
                let Some(disk_key) = disk.as_str() else {
                    return Err(solver_error(format!(
                        "solve layout {layout_name} zfs pool {pool_key} disks entries must be disk keys"
                    )));
                };
                disks
                    .iter()
                    .position(|candidate| candidate.key == disk_key)
                    .ok_or_else(|| {
                        solver_error(format!(
                            "solve layout {layout_name} zfs pool {pool_key} references unknown disk {disk_key}"
                        ))
                    })
            })
            .collect();
    }

    if let Some(tier) = zfs
        .get("tier")
        .or_else(|| zfs.get("mediaTier"))
        .and_then(Value::as_str)
    {
        return Ok(disks
            .iter()
            .enumerate()
            .filter_map(|(index, disk)| (disk.tier == tier).then_some(index))
            .collect());
    }

    Ok((0..disks.len()).collect())
}

fn parse_vdev_shapes(
    layout_name: &str,
    zfs: &Map<String, Value>,
) -> Result<Vec<VdevShape>, PlanDocumentError> {
    let require_redundant = zfs
        .get("vdevs")
        .and_then(|vdevs| vdevs.get("requireRedundant"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    zfs.get("vdevs")
        .and_then(|vdevs| vdevs.get("prefer"))
        .and_then(Value::as_array)
        .map(|preferred| {
            preferred
                .iter()
                .map(|shape| {
                    let Some(kind) = shape.get("type").and_then(Value::as_str) else {
                        return Err(solver_error(format!(
                            "solve layout {layout_name} vdev shapes must declare type"
                        )));
                    };
                    let Some(width) = shape.get("width").and_then(Value::as_u64) else {
                        return Err(solver_error(format!(
                            "solve layout {layout_name} vdev shape {kind} must declare width"
                        )));
                    };
                    if width == 0 {
                        return Err(solver_error(format!(
                            "solve layout {layout_name} vdev shape {kind} width must be at least 1"
                        )));
                    }
                    if require_redundant && !is_redundant_vdev_shape(kind) {
                        return Err(solver_error(format!(
                            "solve layout {layout_name} requires redundancy but vdev shape {kind} is not a redundant ZFS shape"
                        )));
                    }
                    Ok(VdevShape {
                        kind: kind.to_string(),
                        width: width as usize,
                    })
                })
                .collect::<Result<Vec<_>, PlanDocumentError>>()
        })
        .transpose()?
        .filter(|shapes| !shapes.is_empty())
        .map_or_else(
            || {
                Ok(vec![
                    VdevShape {
                        kind: "raidz1".to_string(),
                        width: 3,
                    },
                    VdevShape {
                        kind: "mirror".to_string(),
                        width: 2,
                    },
                ])
            },
            Ok,
        )
}

fn infer_disk_tier(disk: &Value, path: &str, media: &str) -> String {
    if disk
        .get("rotational")
        .and_then(Value::as_bool)
        .is_some_and(|rotational| rotational)
    {
        return "cold".to_string();
    }
    if disk
        .get("solidState")
        .and_then(Value::as_bool)
        .is_some_and(|solid_state| solid_state)
    {
        return "fast".to_string();
    }

    let haystack = format!(
        "{} {} {}",
        media,
        disk.get("transport").and_then(Value::as_str).unwrap_or(""),
        path
    )
    .to_ascii_lowercase();
    if haystack.contains("nvme")
        || haystack.contains("ssd")
        || haystack.contains("solid")
        || haystack.contains("flash")
    {
        "fast".to_string()
    } else if haystack.contains("hdd") || haystack.contains("rotational") {
        "cold".to_string()
    } else {
        "unknown".to_string()
    }
}

fn insert_generated_datasets(generated: &mut Map<String, Value>, zfs: &Map<String, Value>) {
    let Some(datasets) = zfs.get("datasets").and_then(Value::as_object) else {
        return;
    };
    let generated_datasets = collection_mut(generated, "datasets");
    for (name, dataset) in datasets {
        generated_datasets.insert(name.clone(), dataset.clone());
    }
}

fn solve_vdevs(counts: &[u64], shapes: &[VdevShape]) -> Option<VdevSolution> {
    let mut memo = BTreeMap::new();
    solve_vdevs_inner(counts.to_vec(), shapes, &mut memo)
}

fn solve_vdevs_inner(
    counts: Vec<u64>,
    shapes: &[VdevShape],
    memo: &mut BTreeMap<Vec<u64>, Option<VdevSolution>>,
) -> Option<VdevSolution> {
    if let Some(cached) = memo.get(&counts) {
        return cached.clone();
    }

    let mut best = Some(VdevSolution {
        used_slices: 0,
        preferred_shape_counts: vec![0; shapes.len()],
        choices: Vec::new(),
    });

    for (shape_index, shape) in shapes.iter().enumerate() {
        for disk_indexes in disk_combinations(&counts, shape.width) {
            let mut remaining = counts.clone();
            for disk_index in &disk_indexes {
                remaining[*disk_index] -= 1;
            }
            if let Some(mut solution) = solve_vdevs_inner(remaining, shapes, memo) {
                solution.used_slices += shape.width as u64;
                solution.preferred_shape_counts[shape_index] += 1;
                solution.choices.insert(
                    0,
                    VdevChoice {
                        shape_index,
                        disk_indexes,
                    },
                );
                if best
                    .as_ref()
                    .is_none_or(|current| solution_better(&solution, current))
                {
                    best = Some(solution);
                }
            }
        }
    }

    memo.insert(counts, best.clone());
    best
}

fn solution_better(candidate: &VdevSolution, current: &VdevSolution) -> bool {
    candidate
        .used_slices
        .cmp(&current.used_slices)
        .then_with(|| candidate.preferred_shape_counts.cmp(&current.preferred_shape_counts))
        .then_with(|| current.choices.len().cmp(&candidate.choices.len()))
        .then_with(|| current.choices.cmp(&candidate.choices))
        .is_gt()
}

fn disk_combinations(counts: &[u64], width: usize) -> Vec<Vec<usize>> {
    fn collect(
        counts: &[u64],
        width: usize,
        start: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == width {
            output.push(current.clone());
            return;
        }
        for index in start..counts.len() {
            if counts[index] == 0 {
                continue;
            }
            current.push(index);
            collect(counts, width, index + 1, current, output);
            current.pop();
        }
    }

    let mut output = Vec::new();
    collect(counts, width, 0, &mut Vec::new(), &mut output);
    output
}

fn is_redundant_vdev_shape(kind: &str) -> bool {
    matches!(kind, "mirror" | "raidz1" | "raidz2" | "raidz3")
}

fn is_single_vdev_shape(kind: &str) -> bool {
    matches!(kind, "single" | "disk")
}

fn parse_unassigned_slice_policy(value: &str) -> Result<UnassignedSlicePolicy, PlanDocumentError> {
    match value {
        "allow" => Ok(UnassignedSlicePolicy::Allow),
        "forbid" | "forbid-full-slices" => Ok(UnassignedSlicePolicy::Forbid),
        _ => Err(solver_error(format!(
            "unsupported unassigned slice policy {value}; expected allow or forbid"
        ))),
    }
}

fn merge_spec_objects(generated: &mut Map<String, Value>, existing: &Map<String, Value>) {
    for (key, value) in existing {
        if key == "solve" {
            generated.insert(key.clone(), value.clone());
            continue;
        }
        match (generated.get_mut(key), value) {
            (Some(Value::Object(generated_object)), Value::Object(existing_object)) => {
                merge_spec_objects(generated_object, existing_object);
            }
            _ => {
                generated.insert(key.clone(), value.clone());
            }
        }
    }
}

fn solver_error(message: impl Into<String>) -> PlanDocumentError {
    PlanDocumentError::Solver(format!("disk-nix solve failed: {}", message.into()))
}

fn collection_mut<'a>(spec: &'a mut Map<String, Value>, name: &str) -> &'a mut Map<String, Value> {
    spec.entry(name.to_string())
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("generated collection should be a JSON object")
}

fn partition_object(
    device: &str,
    partition_number: u64,
    start: &str,
    end: &str,
    target: &str,
    gpt_type: &str,
    label: Option<&str>,
) -> Value {
    let mut object = Map::new();
    object.insert("operation".to_string(), Value::String("create".to_string()));
    object.insert("device".to_string(), Value::String(device.to_string()));
    object.insert(
        "partitionNumber".to_string(),
        Value::String(partition_number.to_string()),
    );
    object.insert(
        "partitionType".to_string(),
        Value::String("primary".to_string()),
    );
    object.insert("start".to_string(), Value::String(start.to_string()));
    object.insert("end".to_string(), Value::String(end.to_string()));
    object.insert("target".to_string(), Value::String(target.to_string()));
    object.insert("preserveData".to_string(), Value::Bool(false));
    object.insert(
        "metadata".to_string(),
        object_value([("gptType", Value::String(gpt_type.to_string()))]),
    );
    if let Some(label) = label {
        object.insert(
            "properties".to_string(),
            object_value([("label", Value::String(label.to_string()))]),
        );
    }
    Value::Object(object)
}

fn object_value<const N: usize>(entries: [(&str, Value); N]) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn parse_size_gib(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    let number_len = trimmed
        .char_indices()
        .take_while(|(_, character)| character.is_ascii_digit() || *character == '.')
        .map(|(index, character)| index + character.len_utf8())
        .last()?;
    let number = trimmed[..number_len].parse::<f64>().ok()?;
    let unit = trimmed[number_len..].trim().to_ascii_lowercase();
    let gib = match unit.as_str() {
        "mib" | "mi" => number / 1024.0,
        "g" | "gb" | "gib" | "gi" | "" => number,
        "t" | "tb" | "tib" | "ti" => number * 1024.0,
        _ => return None,
    };
    Some(gib.floor() as u64)
}
