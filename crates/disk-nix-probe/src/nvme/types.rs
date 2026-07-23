#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NvmeList {
    #[serde(default)]
    devices: Vec<NvmeDevice>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NvmeDevice {
    #[serde(alias = "Name")]
    device_path: Option<String>,
    #[serde(alias = "GenericDevice")]
    generic: Option<String>,
    model_number: Option<String>,
    product_name: Option<String>,
    serial_number: Option<String>,
    #[serde(alias = "FirmwareRevision", alias = "FWRev")]
    firmware: Option<String>,
    index: Option<u64>,
    #[serde(alias = "NameSpace", alias = "Namespace")]
    namespace: Option<u64>,
    #[serde(alias = "NSID")]
    namespace_id: Option<u64>,
    #[serde(alias = "NamespaceUUID", alias = "NSUUID")]
    namespace_uuid: Option<String>,
    #[serde(alias = "EUI64")]
    eui64: Option<String>,
    #[serde(alias = "NGUID")]
    nguid: Option<String>,
    #[serde(rename = "SubSystem", alias = "Subsystem", alias = "SubsystemNQN")]
    subsystem: Option<String>,
    #[serde(alias = "Controller")]
    controller: Option<String>,
    #[serde(alias = "Address", alias = "TransportAddress")]
    address: Option<String>,
    #[serde(alias = "NamespaceSize")]
    physical_size: Option<u64>,
    #[serde(alias = "NamespaceUsage")]
    used_bytes: Option<u64>,
    #[serde(alias = "NamespaceCapacity")]
    namespace_capacity: Option<u64>,
    maximum_lba: Option<u64>,
    sector_size: Option<u64>,
    #[serde(alias = "Transport", alias = "TrType")]
    transport: Option<String>,
    #[serde(alias = "ControllerID", alias = "CNTLID")]
    controller_id: Option<u64>,
    #[serde(alias = "Format", alias = "LBAFormat")]
    lba_format: Option<String>,
    #[serde(alias = "ANAState")]
    ana_state: Option<String>,
}
