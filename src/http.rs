use std::{collections::BTreeMap, net::IpAddr, os::unix::prelude::AsRawFd, str::Utf8Error};

use smoltcp::{
    iface::{InterfaceBuilder, NeighborCache, Routes},
    phy::{self, TunTapInterface},
    socket::{TcpSocket, TcpSocketBuffer},
    time::Instant,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address},
};
use url::Url;

#[derive(Debug)]
enum HttpState {
    Connect,
    Request,
    Response,
}

#[derive(Debug)]
pub enum UpstreamError {
    Network(smoltcp::Error),
    InvalidUrl,
    Content(Utf8Error),
}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self) // Using Debug
    }
}

impl From<smoltcp::Error> for UpstreamError {
    fn from(error: smoltcp::Error) -> Self {
        UpstreamError::Network(error)
    }
}

impl From<std::str::Utf8Error> for UpstreamError {
    fn from(error: std::str::Utf8Error) -> Self {
        UpstreamError::Content(error)
    }
}

fn random_port() -> u16 {
    // A random port between 49152 and 65535
    // get_ephemeral_port() -> u16
    49152 + rand::random::<u16>() % 16384
}

pub fn get(
    tap: TunTapInterface,
    mac: EthernetAddress,
    addr: IpAddr,
    url: Url,
) -> Result<(), UpstreamError> {
    let domain_name = url.host_str().ok_or(UpstreamError::InvalidUrl)?;
    // A neighbor mapping translates from a protocol address to a 
    // hardware address, and contains the timestamp past which the 
    // mapping should be discarded.
    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let mac = HardwareAddress::Ethernet(mac);

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
    // Classless Inter-Domain Routing (CIDR) is a 
    // method for allocating IP addresses and for IP routing.
    let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 42, 1), 24)];
    // TUN and TAP are kernel virtual network devices. Being network 
    // devices supported entirely in software, they differ from 
    // ordinary network devices which are backed by physical network 
    // adapters. 
    let fd = tap.as_raw_fd();
    let mut routes = Routes::new(BTreeMap::new());
    let default_gateway = Ipv4Address::new(192, 168, 42, 100);
    routes.add_default_ipv4_route(default_gateway).unwrap();

    // let mut sockets = SocketSet::new(vec![]);

    let mut iface = InterfaceBuilder::new(tap, vec![])
        .hardware_addr(mac)
        .neighbor_cache(neighbor_cache)
        .ip_addrs(ip_addrs)
        .routes(routes)
        .finalize();
    let tcp_handle = iface.add_socket(tcp_socket);

    let http_header = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url.path(),
        domain_name
    );

    let mut state = HttpState::Connect;

    'http: loop {
        let timestamp = Instant::now();
        match iface.poll(timestamp) {
            Ok(_) => {}
            Err(smoltcp::Error::Unrecognized) => {}
            Err(e) => {
                eprintln!("ERROR: {:?}", e)
            }
        }
        {
            let (socket, cx) = iface.get_socket_and_context::<TcpSocket>(tcp_handle);
            state = match state {
                HttpState::Connect if !socket.is_active() => {
                    eprintln!("connecting");
                    socket.connect(cx, (addr, 80), random_port())?;
                    HttpState::Request
                }
                HttpState::Request if socket.may_send() => {
                    eprintln!("sending request");
                    socket.send_slice(http_header.as_ref())?;
                    HttpState::Response
                }
                HttpState::Response if socket.can_recv() => {
                    socket.recv(|raw_data| {
                        // let raw_data = raw_data.to_owned();
                        let output = String::from_utf8_lossy(raw_data);
                        println!("{}", output);
                        (raw_data.len(), ())
                    })?;
                    HttpState::Response
                }
                HttpState::Response if !socket.may_recv() => {
                    eprintln!("received complete response");
                    break 'http;
                }
                _ => state,
            }
        }
        phy::wait(fd, iface.poll_delay(timestamp)).expect("wait error");
    }
    Ok(())
}
