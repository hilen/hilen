use std::{collections::BTreeMap, net::IpAddr, time::Duration};

use anyhow::{Result, bail};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::time::timeout;

use super::{
    NetService, ServiceBrowser, ServiceEvent,
    service::{instance_name, parse_text_record, service_type, sort_addresses},
};

/// How long the test waits for the network.
const WAIT: Duration = Duration::from_secs(20);

#[test]
fn a_service_type_is_taken_with_or_without_its_domain() {
    assert_eq!(service_type("_turnscan._tcp").unwrap(), "_turnscan._tcp");
    assert_eq!(service_type("_turnscan._tcp.").unwrap(), "_turnscan._tcp");
    assert_eq!(service_type("_turnscan._tcp.local.").unwrap(), "_turnscan._tcp");
    assert_eq!(service_type("_board._udp.local").unwrap(), "_board._udp");
    for wrong in [
        "",
        "turnscan._tcp",
        "_turnscan",
        "_turnscan._xyz",
        "_._tcp",
        "_a._b._tcp",
    ] {
        assert!(service_type(wrong).is_err(), "{wrong:?} was taken");
        assert!(
            ServiceBrowser::start(wrong).is_err(),
            "{wrong:?} started a browse"
        );
    }
}

#[test]
fn addresses_of_the_real_network_come_first() {
    let mut addresses: Vec<IpAddr> = [
        "fe80::1",
        "169.254.7.85",
        "2a02::5",
        "192.168.0.208",
        "192.168.0.208",
    ]
    .iter()
    .map(|text| text.parse().unwrap())
    .collect();
    sort_addresses(&mut addresses);
    let texts: Vec<String> = addresses.iter().map(ToString::to_string).collect();
    assert_eq!(texts, ["192.168.0.208", "169.254.7.85", "2a02::5", "fe80::1"]);

    let service = NetService {
        name: "Board".into(),
        host: "board.local.".into(),
        addresses,
        port: 80,
        text: BTreeMap::new(),
    };
    assert_eq!(service.socket_address().to_string(), "192.168.0.208:80");
}

#[test]
fn a_text_record_is_read_whole() {
    let text = parse_text_record(b"\x05a=one\x04flag\x00\x06empty=\x0dapp_id=ABCDEF");
    let pairs: Vec<(&str, &str)> = text.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
    assert_eq!(
        pairs,
        [("a", "one"), ("app_id", "ABCDEF"), ("empty", ""), ("flag", "")]
    );
    // An entry longer than the record ends it.
    assert_eq!(parse_text_record(b"\x03a=1\x20app_id=A").len(), 1);
}

#[test]
fn an_instance_name_is_cut_out_of_its_full_name() {
    let full_type = "_turnscan._tcp.local.";
    assert_eq!(
        instance_name("Board 2._turnscan._tcp.local.", full_type),
        "Board 2"
    );
    assert_eq!(instance_name("other.local.", full_type), "other.local.");
}

/// A service announced with `mdns-sd`, like a board does, is found with its
/// port and text, and lost when it goes.
#[tokio::test]
async fn an_announced_service_is_found_and_lost() -> Result<()> {
    let number = fastrand::u32(100_000..1_000_000);
    let full_type = format!("_hilen{number}._tcp.local.");
    let name = format!("Test board {number}");

    let mut browser = ServiceBrowser::start(&full_type)?;
    assert_eq!(browser.service_type(), format!("_hilen{number}._tcp"));

    let board = ServiceDaemon::new()?;
    let info = ServiceInfo::new(
        &full_type,
        &name,
        &format!("hilen-test-{number}.local."),
        "",
        4242,
        &[("board", "2"), ("firmware", "1.0")][..],
    )?
    .enable_addr_auto();
    let full_name = info.get_fullname().to_string();
    board.register(info)?;

    let found = timeout(WAIT, async {
        loop {
            if let ServiceEvent::Found(service) = browser.next_event().await?
                && service.name == name
            {
                return Ok::<_, anyhow::Error>(service);
            }
        }
    })
    .await??;
    assert_eq!(found.port, 4242);
    assert_eq!(found.text.get("board").map(String::as_str), Some("2"));
    assert_eq!(found.text.get("firmware").map(String::as_str), Some("1.0"));
    assert_ne!(found.addresses, Vec::<IpAddr>::new());

    board.unregister(&full_name)?;
    timeout(WAIT, async {
        loop {
            match browser.next_event().await? {
                ServiceEvent::Lost(lost) if lost == name => return Ok::<_, anyhow::Error>(()),
                ServiceEvent::Lost(_) | ServiceEvent::Found(_) => {}
            }
        }
    })
    .await??;

    if let Err(error) = board.shutdown() {
        bail!("the board daemon did not stop: {error}");
    }
    Ok(())
}
