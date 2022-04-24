use std::{
    net::{SocketAddr, UdpSocket},
    time::Duration,
};

use trust_dns_client::{
    op::{Message, Query},
    proto::error::ProtoError,
    rr::domain::Name,
    serialize::binary::*,
};

fn message_id() -> u16 {
    let candidate = rand::random();
    if candidate == 0 {
        return message_id();
    }
    candidate
}
#[derive(Debug)]
pub enum DnsError {
    ParseDomainName(ProtoError),
    ParseDnsServerAddress(std::net::AddrParseError),
    Encoding(ProtoError),
    Decoding(ProtoError),
    Network(std::io::Error),
    Sending(std::io::Error),
    Receiving(std::io::Error),
}
impl std::fmt::Display for DnsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#?}", self)
    }
}
impl std::error::Error for DnsError {}

pub fn resolve(
    dns_server_address: &str,
    domain_name: &str,
) -> Result<Option<std::net::IpAddr>, Box<dyn std::error::Error>> {
    let domain_name = Name::from_ascii(domain_name).map_err(DnsError::ParseDomainName)?;
    let dns_server_address = format!("{}:53", dns_server_address);

    let dns_server: SocketAddr = dns_server_address
        .parse()
        .map_err(DnsError::ParseDnsServerAddress)?;

    let mut request_buffer = Vec::with_capacity(64);
    let mut response_buffer = vec![0u8; 512];

    let mut request = Message::new();
    request
        .add_query(Query::query(
            domain_name,
            trust_dns_client::rr::RecordType::A,
        ))
        .set_id(message_id())
        .set_message_type(trust_dns_client::op::MessageType::Query)
        .set_op_code(trust_dns_client::op::OpCode::Query)
        .set_recursion_desired(true);

    let localhost = UdpSocket::bind("0.0.0.0:0").map_err(DnsError::Network)?;

    let timeout = Duration::from_secs(5);

    localhost
        .set_read_timeout(Some(timeout))
        .map_err(DnsError::Network)?;

    localhost
        .set_nonblocking(false)
        .map_err(DnsError::Network)?;

    let mut encoder = BinEncoder::new(&mut request_buffer);
    request.emit(&mut encoder).map_err(DnsError::Encoding)?;
    let _n_bytes_sent = localhost
        .send_to(&request_buffer, dns_server)
        .map_err(DnsError::Sending)?;

    loop {
        let (_b_bytes_recv, remote_port) = localhost
            .recv_from(&mut response_buffer)
            .map_err(DnsError::Receiving)?;
        if remote_port == dns_server {
            break;
        }
    }

    let response = Message::from_vec(&response_buffer).map_err(DnsError::Decoding)?;
    for answer in response.answers() {
        if answer.record_type() == trust_dns_client::rr::RecordType::A {
            let resource = answer.data().unwrap();
            let server_ip = resource.to_ip_addr().expect("invalid IP address received");
            return Ok(Some(server_ip));
        }
    }
    Ok(None)
}
