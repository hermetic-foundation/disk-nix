struct ZpoolRow {
    name: String,
    size: Option<u64>,
    allocated: Option<u64>,
    free: Option<u64>,
    health: Option<String>,
    capacity: Option<String>,
    dedupratio: Option<String>,
    fragmentation: Option<String>,
    altroot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolProperty {
    pool: String,
    property: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZfsRow {
    name: String,
    kind: String,
    used: Option<u64>,
    available: Option<u64>,
    referenced: Option<u64>,
    mountpoint: Option<String>,
    origin: Option<String>,
    userrefs: Option<String>,
    compression: Option<String>,
    quota: Option<String>,
    reservation: Option<String>,
    encryption: Option<String>,
    keystatus: Option<String>,
    volsize: Option<String>,
    recordsize: Option<String>,
    dedup: Option<String>,
    checksum: Option<String>,
    copies: Option<String>,
    sync: Option<String>,
    primarycache: Option<String>,
    secondarycache: Option<String>,
    atime: Option<String>,
    relatime: Option<String>,
    snapdir: Option<String>,
    acltype: Option<String>,
    xattr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZfsHold {
    snapshot: String,
    tag: String,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolStatus {
    name: String,
    state: Option<String>,
    status: Option<String>,
    action: Option<String>,
    scan: Option<String>,
    errors: Option<String>,
    read_errors: Option<String>,
    write_errors: Option<String>,
    checksum_errors: Option<String>,
    vdevs: Vec<ZpoolVdev>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ZpoolVdev {
    name: String,
    role: String,
    parent: Option<String>,
    state: Option<String>,
    read_errors: Option<String>,
    write_errors: Option<String>,
    checksum_errors: Option<String>,
    device_path: Option<String>,
}
