use disk_nix_model::{Edge, Node, NodeKind, Relationship, StorageGraph};

use crate::ProbeError;

include!("iscsi/records.rs");
include!("iscsi/sessions.rs");
include!("iscsi/nodes.rs");
include!("iscsi/graph.rs");
include!("iscsi/helpers.rs");

#[cfg(test)]
mod tests {
    use disk_nix_model::{NodeKind, Relationship};

    use super::*;

    const SESSION: &[u8] = br#"
Target: iqn.2026-06.example:storage.disk1
    Current Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.10:3260,1
    Target Portal Group Tag: 1
    **********
    Interface:
    **********
    Iface Name: default
    Iface Transport: tcp
    Iface Initiatorname: iqn.2026-06.client:node1
    Iface IPaddress: 10.0.0.20
    Iface Netdev: eno1
    SID: 12
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    HeaderDigest: None
    DataDigest: None
    MaxRecvDataSegmentLength: 262144
    MaxBurstLength: 262144
    CID: 0
    Connection State: LOGGED IN
    Local Address: 10.0.0.20
    Peer Address: 10.0.0.10
    Host Number: 4  State: running
    scsi4 Channel 00 Id 0 Lun: 0
        Attached scsi disk sdb          State: running
"#;

    const ISER_SESSION: &[u8] = br#"
Target: iqn.2026-06.example:rdma.archive
    Current Portal: [2001:db8:40::10]:3260,2
    Persistent Portal: [2001:db8:40::10]:3260,2
    Target Portal Group Tag: 2
    **********
    Interface:
    **********
    Iface Name: iser-rdma0
    Iface Transport: iser
    Iface Initiatorname: iqn.2026-06.client:rdma-node
    Iface IPaddress: 2001:db8:40::20
    Iface Netdev: ib0
    SID: 44
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    HeaderDigest: None
    DataDigest: None
    MaxRecvDataSegmentLength: 1048576
    MaxXmitDataSegmentLength: 1048576
    FirstBurstLength: 262144
    MaxBurstLength: 1048576
    ImmediateData: Yes
    InitialR2T: No
    MaxOutstandingR2T: 1
    CID: 0
    Connection State: LOGGED IN
    Local Address: 2001:db8:40::20
    Peer Address: 2001:db8:40::10
    Host Number: 15  State: running
    scsi15 Channel 00 Id 0 Lun: 3
        Attached scsi disk sdg          State: running
"#;

    #[test]
    fn normalizes_iscsi_session_target_lun_and_disk() {
        let graph = normalize_iscsi_session_output(SESSION).expect("fixture should parse");

        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::IscsiSession && node.name == "iscsi-session:12"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == NodeKind::IscsiTarget));
        assert!(graph.nodes.iter().any(|node| node.kind == NodeKind::Lun));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiSession
                && node.name == "iscsi-session:12"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.target"
                        && property.value == "iqn.2026-06.example:storage.disk1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal"
                        && property.value == "10.0.0.10:3260,1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.portal-address" && property.value == "10.0.0.10"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-port" && property.value == "3260")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-tpgt" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-address"
                        && property.value == "10.0.0.10"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-port" && property.value == "3260"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.persistent-portal-tpgt" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-state" && property.value == "LOGGED IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.target-portal-group-tag" && property.value == "1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.session-state" && property.value == "LOGGED_IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.iface-transport" && property.value == "tcp"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.iface-initiator-name"
                        && property.value == "iqn.2026-06.client:node1"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.host-number" && property.value == "4")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.maxrecvdatasegmentlength" && property.value == "262144"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.connection-cid" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-detail-state" && property.value == "LOGGED IN"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-local-address"
                        && property.value == "10.0.0.20"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-peer-address" && property.value == "10.0.0.10"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Lun
                && node.name == "0"
                && node.path.as_deref() == Some("/dev/sdb")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-channel" && property.value == "00")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-id" && property.value == "0")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.attached-disk-state" && property.value == "running"
                })
        }));
        assert!(graph.edges.iter().any(|edge| edge.from.0
            == "iscsi-lun:iqn.2026-06.example:storage.disk1:0"
            && edge.to.0 == "block:/dev/sdb"
            && edge.relationship == Relationship::Backs));
    }

    #[test]
    fn normalizes_iser_ipv6_session_fixture() {
        let graph = normalize_iscsi_session_output(ISER_SESSION).expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiSession
                && node.name == "iscsi-session:44"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.target"
                        && property.value == "iqn.2026-06.example:rdma.archive"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.iface-transport" && property.value == "iser"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.iface-netdev" && property.value == "ib0")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.portal-address" && property.value == "2001:db8:40::10"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-port" && property.value == "3260")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.portal-tpgt" && property.value == "2")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.maxxmitdatasegmentlength" && property.value == "1048576"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.initialr2t" && property.value == "No")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-local-address"
                        && property.value == "2001:db8:40::20"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.connection-peer-address"
                        && property.value == "2001:db8:40::10"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Lun
                && node.name == "3"
                && node.path.as_deref() == Some("/dev/sdg")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.host-number" && property.value == "15")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-id" && property.value == "0")
        }));
        assert!(graph.edges.iter().any(|edge| edge.from.0
            == "iscsi-lun:iqn.2026-06.example:rdma.archive:3"
            && edge.to.0 == "block:/dev/sdg"
            && edge.relationship == Relationship::Backs));
    }

    #[test]
    fn parses_bracketed_ipv6_iscsi_portal_parts() {
        assert_eq!(
            portal_parts("iscsi.portal", "[2001:db8::10]:3260,2"),
            vec![
                (
                    "iscsi.portal-address".to_string(),
                    "2001:db8::10".to_string()
                ),
                ("iscsi.portal-port".to_string(), "3260".to_string()),
                ("iscsi.portal-tpgt".to_string(), "2".to_string())
            ]
        );
        assert_eq!(
            portal_parts("iscsi.portal", "2001:db8::10"),
            vec![(
                "iscsi.portal-address".to_string(),
                "2001:db8::10".to_string()
            )]
        );
    }

    #[test]
    fn normalizes_ipv6_session_and_concise_node_fixture() {
        let session_graph = normalize_iscsi_session_output(
            br#"
Target: iqn.2026-06.example:storage.ipv6
    Current Portal: [2001:db8::10]:3260,2
    Persistent Portal: [2001:db8::11]:3260,2
    Target Portal Group Tag: 2
    SID: 44
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Host Number: 8  State: running
    scsi8 Channel 00 Id 1 Lun: 3
        Attached scsi disk sdd          State: running
"#,
        )
        .expect("IPv6 iSCSI session fixture should parse");
        let session = session_graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:44")
            .expect("IPv6 iSCSI session should exist");
        assert!(session.properties.iter().any(|property| {
            property.key == "iscsi.portal" && property.value == "[2001:db8::10]:3260,2"
        }));
        assert!(session.properties.iter().any(|property| {
            property.key == "iscsi.portal-address" && property.value == "2001:db8::10"
        }));
        assert!(session
            .properties
            .iter()
            .any(|property| property.key == "iscsi.portal-port" && property.value == "3260"));
        assert!(session
            .properties
            .iter()
            .any(|property| property.key == "iscsi.portal-tpgt" && property.value == "2"));
        assert!(session.properties.iter().any(|property| {
            property.key == "iscsi.persistent-portal-address" && property.value == "2001:db8::11"
        }));
        assert!(session
            .properties
            .iter()
            .any(|property| property.key == "iscsi.host-number" && property.value == "8"));
        assert!(session_graph.nodes.iter().any(|node| {
            node.kind == NodeKind::Lun
                && node.id.0 == "iscsi-lun:iqn.2026-06.example:storage.ipv6:3"
                && node.path.as_deref() == Some("/dev/sdd")
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.scsi-id" && property.value == "1")
        }));
        assert!(session_graph.edges.iter().any(|edge| {
            edge.from.0 == "iscsi-lun:iqn.2026-06.example:storage.ipv6:3"
                && edge.to.0 == "block:/dev/sdd"
                && edge.relationship == Relationship::Backs
        }));

        let node_graph = normalize_iscsi_node_output(
            br#"
[2001:db8::12]:3260,2 iqn.2026-06.example:storage.ipv6
Target: iqn.2026-06.example:storage.chap
    Portal: [2001:db8::13]:3260,3
    node.startup: automatic
    node.session.auth.authmethod: CHAP
    node.session.auth.username: host-user
    node.session.auth.password: []
    node.session.auth.username_in: target-user
    node.session.auth.password_in: inbound-secret
"#,
        )
        .expect("IPv6 iSCSI node fixture should parse");
        assert!(node_graph.nodes.iter().any(|node| {
            node.id.0 == "iscsi-target:iqn.2026-06.example:storage.ipv6"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal" && property.value == "[2001:db8::12]:3260,2"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal-address" && property.value == "2001:db8::12"
                })
        }));
        let chap_node = node_graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-target:iqn.2026-06.example:storage.chap")
            .expect("CHAP iSCSI node should exist");
        assert!(chap_node.properties.iter().any(|property| {
            property.key == "iscsi.node-auth-username" && property.value == "host-user"
        }));
        assert!(!chap_node
            .properties
            .iter()
            .any(|property| { property.key == "iscsi.node-auth-password-configured" }));
        assert!(chap_node.properties.iter().any(|property| {
            property.key == "iscsi.node-auth-password-in-configured" && property.value == "true"
        }));
        assert!(!chap_node
            .properties
            .iter()
            .any(|property| property.value == "inbound-secret"));
    }

    #[test]
    fn normalizes_multi_portal_discovery_auth_and_lun_churn_fixture() {
        let mut graph = StorageGraph::empty();
        merge_test_graph(
            &mut graph,
            normalize_iscsi_session_output(
                br#"
Target: iqn.2026-06.example:storage.churn
    Current Portal: 10.30.0.10:3260,1
    Persistent Portal: 10.30.0.10:3260,1
    Target Portal Group Tag: 1
    Iface Name: vlan10
    Iface Transport: tcp
    SID: 71
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    CID: 0
    Connection State: LOGGED IN
    Local Address: 10.30.0.21
    Peer Address: 10.30.0.10
    Host Number: 21  State: running
    scsi21 Channel 00 Id 0 Lun: 5
        Attached scsi disk sdh          State: running
Target: iqn.2026-06.example:storage.churn
    Current Portal: 10.30.0.11:3260,2
    Persistent Portal: 10.30.0.11:3260,2
    Target Portal Group Tag: 2
    Iface Name: vlan11
    Iface Transport: tcp
    SID: 72
    iSCSI Connection State: FREE
    iSCSI Session State: FAILED
    Internal iscsid Session State: REOPEN
    CID: 0
    Connection State: FREE
    Local Address: 10.30.0.22
    Peer Address: 10.30.0.11
    Host Number: 22  State: blocked
    scsi22 Channel 00 Id 0 Lun: 5
        Attached scsi disk sdg          State: transport-offline
Target: iqn.2026-06.example:storage.replacement
    Current Portal: 10.30.0.12:3260,3
    Persistent Portal: 10.30.0.12:3260,3
    Target Portal Group Tag: 3
    SID: 73
    iSCSI Connection State: LOGGED IN
    iSCSI Session State: LOGGED_IN
    Internal iscsid Session State: NO CHANGE
    Host Number: 23  State: running
    scsi23 Channel 00 Id 0 Lun: 8
        Attached scsi disk sdi          State: running
"#,
            )
            .expect("multi-portal iSCSI session fixture should parse"),
        );
        merge_test_graph(
            &mut graph,
            normalize_iscsi_node_output(
                br#"
Target: iqn.2026-06.example:storage.churn
    Portal: 10.30.0.10:3260,1
    Persistent Portal: 10.30.0.11:3260,2
    TPGT: 1
    Iface Name: vlan10
    Startup: automatic
    Leading Login: Yes
    AuthMethod: CHAP
    Username: outbound-user
    Password: outbound-secret
    Username_in: inbound-user
    Password_in: inbound-secret
    discovery.sendtargets.auth.authmethod: CHAP
    discovery.sendtargets.auth.username: discovery-user
    discovery.sendtargets.auth.password: discovery-secret
    discovery.sendtargets.auth.username_in: discovery-target
    discovery.sendtargets.auth.password_in: discovery-in-secret
Target: iqn.2026-06.example:storage.replacement
    Portal: 10.30.0.12:3260,3
    Startup: manual
    AuthMethod: None
"#,
            )
            .expect("multi-portal iSCSI node fixture should parse"),
        );

        let primary = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:71")
            .expect("primary portal session should exist");
        assert!(primary.properties.iter().any(|property| {
            property.key == "iscsi.portal" && property.value == "10.30.0.10:3260,1"
        }));
        assert!(primary.properties.iter().any(|property| {
            property.key == "iscsi.session-state" && property.value == "LOGGED_IN"
        }));

        let stale = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-session:72")
            .expect("stale portal session should exist");
        assert!(stale.properties.iter().any(|property| {
            property.key == "iscsi.portal" && property.value == "10.30.0.11:3260,2"
        }));
        assert!(stale.properties.iter().any(|property| {
            property.key == "iscsi.session-state" && property.value == "FAILED"
        }));
        assert!(stale.properties.iter().any(|property| {
            property.key == "iscsi.internal-session-state" && property.value == "REOPEN"
        }));
        assert!(stale
            .properties
            .iter()
            .any(|property| { property.key == "iscsi.host-state" && property.value == "blocked" }));

        let target = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-target:iqn.2026-06.example:storage.churn")
            .expect("configured churn target should exist");
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.node-auth-method" && property.value == "CHAP"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.node-auth-password-configured" && property.value == "true"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.node-auth-password-in-configured" && property.value == "true"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.discovery-auth-method" && property.value == "CHAP"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.discovery-auth-username" && property.value == "discovery-user"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.discovery-auth-password-configured" && property.value == "true"
        }));
        assert!(target.properties.iter().any(|property| {
            property.key == "iscsi.discovery-auth-password-in-configured"
                && property.value == "true"
        }));
        assert!(!target.properties.iter().any(|property| {
            matches!(
                property.value.as_str(),
                "outbound-secret" | "inbound-secret" | "discovery-secret" | "discovery-in-secret"
            )
        }));

        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "iscsi-lun:iqn.2026-06.example:storage.churn:5"
                && node.path.as_deref() == Some("/dev/sdh")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.attached-disk-state" && property.value == "running"
                })
        }));
        let churn_lun = graph
            .nodes
            .iter()
            .find(|node| node.id.0 == "iscsi-lun:iqn.2026-06.example:storage.churn:5")
            .expect("merged churn LUN identity should exist");
        assert!(churn_lun
            .properties
            .iter()
            .any(|property| { property.key == "iscsi.attached-disk" && property.value == "sdh" }));
        assert!(churn_lun
            .properties
            .iter()
            .any(|property| { property.key == "iscsi.attached-disk" && property.value == "sdg" }));
        assert!(churn_lun.properties.iter().any(|property| {
            property.key == "iscsi.attached-disk-state" && property.value == "running"
        }));
        assert!(churn_lun.properties.iter().any(|property| {
            property.key == "iscsi.attached-disk-state" && property.value == "transport-offline"
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.id.0 == "iscsi-lun:iqn.2026-06.example:storage.replacement:8"
                && node.path.as_deref() == Some("/dev/sdi")
        }));
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.to.0 == "iscsi-target:iqn.2026-06.example:storage.churn"
                        && edge.relationship == Relationship::ImportedFrom
                })
                .count(),
            2
        );
    }

    fn merge_test_graph(graph: &mut StorageGraph, other: StorageGraph) {
        for node in other.nodes {
            graph.add_node(node);
        }
        for edge in other.edges {
            graph.add_edge(edge);
        }
    }

    #[test]
    fn normalizes_configured_iscsi_nodes() {
        let graph = normalize_iscsi_node_output(
            br#"
Target: iqn.2026-06.example:storage.disk1
    Portal: 10.0.0.10:3260,1
    Persistent Portal: 10.0.0.11:3260,1
    TPGT: 1
    Iface Name: default
    Startup: automatic
    Leading Login: Yes
    AuthMethod: CHAP
    Username: node-user
    Password: outbound-secret
    Username_in: target-user
    Password_in: inbound-secret
10.0.0.12:3260,2 iqn.2026-06.example:storage.disk2
"#,
        )
        .expect("fixture should parse");

        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiTarget
                && node.name == "iqn.2026-06.example:storage.disk1"
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.node-configured")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal" && property.value == "10.0.0.10:3260,1"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal-address" && property.value == "10.0.0.10"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-persistent-portal"
                        && property.value == "10.0.0.11:3260,1"
                })
                && node
                    .properties
                    .iter()
                    .any(|property| property.key == "iscsi.node-tpgt" && property.value == "1")
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-startup" && property.value == "automatic"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-method" && property.value == "CHAP"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-username" && property.value == "node-user"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-username-in" && property.value == "target-user"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-password-configured"
                        && property.value == "true"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-password-in-configured"
                        && property.value == "true"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-direction-out" && property.value == "true"
                })
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-auth-direction-in" && property.value == "true"
                })
                && !node.properties.iter().any(|property| {
                    property.value == "outbound-secret" || property.value == "inbound-secret"
                })
        }));
        assert!(graph.nodes.iter().any(|node| {
            node.kind == NodeKind::IscsiTarget
                && node.name == "iqn.2026-06.example:storage.disk2"
                && node.properties.iter().any(|property| {
                    property.key == "iscsi.node-portal" && property.value == "10.0.0.12:3260,2"
                })
        }));
    }
}
