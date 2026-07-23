include!("capabilities/core.rs");
include!("capabilities/filesystems.rs");
include!("capabilities/logical_volumes.rs");

#[must_use]
pub fn default_capabilities() -> Vec<Capability> {
    let mut capabilities = Vec::new();
    capabilities.extend(capability_group_core());
    capabilities.extend(capability_group_filesystems());
    capabilities.extend(capability_group_logical_volumes());
    capabilities
}
