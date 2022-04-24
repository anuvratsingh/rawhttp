use clap::{Arg, Command};
use smoltcp::phy::{Medium, TunTapInterface};
use url::Url;
mod dns;
mod ethernet;
mod http;

fn main() {
    let app = Command::new("mget")
        .about("GET a webpage manually")
        .arg(Arg::new("url").required(true))
        .arg(Arg::new("tap-device").required(true))
        .arg(Arg::new("dns-server").default_value("1.1.1.1"))
        .get_matches();

    let url_input = app.value_of("url").unwrap();
    let dns_server_text = app.value_of("dns-server").unwrap();
    
    let tap_text = app.value_of("tap-device").unwrap();

    let url = Url::parse(url_input).expect("ERROR: Invalid URL");

    if url.scheme() != "http" {
        eprintln!("ERROR: Only \"HTTP\" protocol is supported");
        return;
    }

    let tap = TunTapInterface::new(&tap_text, Medium::Ethernet)
        .expect("ERROR: Unable to use <tap-device> as a network interface");
    let domain_name = url.host_str().expect("Error: Domain Name not provided");

    let _dns_server: std::net::Ipv4Addr = dns_server_text
        .parse()
        .expect("Error: Unable to parse <dns-server> as an IPv4 address");
    let addr = dns::resolve(dns_server_text, domain_name).unwrap().unwrap();

    let mac = ethernet::MacAddress::new().into();

    http::get(tap, mac, addr, url).unwrap();
}
