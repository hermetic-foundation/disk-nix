use std::{collections::BTreeSet, fs, process::Command};

use disk_nix_model::StorageGraph;
use serde::{Serialize, ser::SerializeStruct};
use thiserror::Error;

mod bcache;
mod bcachefs;
mod blkid;
mod btrfs;
mod cryptsetup;
mod dmsetup;
mod exfat;
mod ext;
mod f2fs;
mod findmnt;
mod iscsi;
mod loopdev;
mod lsblk;
mod lsscsi;
mod lvm;
mod mdraid;
mod multipath;
mod nfs;
mod ntfs;
mod nvme;
mod parted;
mod smartctl;
mod swaps;
mod udev;
mod vdo;
mod xfs;
mod zfs;
mod zram;

include!("sections/model.rs");
include!("sections/block_collectors.rs");
include!("sections/filesystem_collectors.rs");
include!("sections/storage_collectors.rs");
include!("sections/network_collectors.rs");
include!("sections/lvm_remediation.rs");
include!("sections/util.rs");

#[cfg(test)]
mod tests;
