#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiSession {
    id: String,
    target: Option<String>,
    portal: Option<String>,
    persistent_portal: Option<String>,
    target_portal_group_tag: Option<String>,
    connection_state: Option<String>,
    session_state: Option<String>,
    internal_session_state: Option<String>,
    iface_name: Option<String>,
    iface_transport: Option<String>,
    iface_initiator_name: Option<String>,
    iface_ip_address: Option<String>,
    iface_netdev: Option<String>,
    host_number: Option<String>,
    host_state: Option<String>,
    connection_params: Vec<(String, String)>,
    negotiated_params: Vec<(String, String)>,
    luns: Vec<IscsiLun>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiNodeRecord {
    target: String,
    portal: Option<String>,
    persistent_portal: Option<String>,
    target_portal_group_tag: Option<String>,
    iface_name: Option<String>,
    startup: Option<String>,
    leading_login: Option<String>,
    auth_method: Option<String>,
    username: Option<String>,
    username_in: Option<String>,
    password_configured: bool,
    password_in_configured: bool,
    discovery_auth_method: Option<String>,
    discovery_username: Option<String>,
    discovery_username_in: Option<String>,
    discovery_password_configured: bool,
    discovery_password_in_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IscsiLun {
    lun: String,
    attached_device: Option<String>,
    attached_device_state: Option<String>,
    host_number: Option<String>,
    scsi_channel: Option<String>,
    scsi_id: Option<String>,
}
