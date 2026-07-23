fn rollback_recipe_safety_gates() -> Vec<String> {
    vec![
        "original apply receipt must match this failed apply report".to_string(),
        "fresh topology probe must be captured after the failure".to_string(),
        "expected, pre-apply, failed-apply, and current topology evidence must be bound before automated rollback".to_string(),
        "rollback point identity must still match the failed action target".to_string(),
        "active consumers, mounts, exports, sessions, or open mappings must be reviewed before any mutation".to_string(),
        "missing tools, stale identity data, and ambiguous rollback targets keep the recipe review-only".to_string(),
        "filesystem rollback gates require verified ext, XFS, FAT, exFAT, NTFS, f2fs, mount/remount, trim, scrub, repair, grow, and shrink state before mutation".to_string(),
        "block-stack rollback gates require verified disk label, partition, LUKS, LVM, MD RAID, device-mapper, loop, backing-file, swap, and zram topology before mutation".to_string(),
        "advanced-storage rollback gates require verified ZFS, Btrfs, bcachefs, bcache, LVM cache, VDO, snapshot, clone, and pool-membership topology before mutation".to_string(),
        "network-storage rollback gates require verified NFS, iSCSI, multipath, NVMe-oF, host-side LUN, and target-side LUN provider topology before mutation".to_string(),
    ]
}
