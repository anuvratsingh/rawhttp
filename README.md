A cli tool requesting and responding to raw http 1.0 requests.


### Run
1. Create a virtual networking device 
```
sudo ip tuntap add mode tap name <tap-device-name> user <user-name>
sudo ip link set <tap-device-name> up
sudo ip addr add 192.168.42.100/24 dev <test-device-name>
sudo iptables -t nat -A POSTROUTING -s 192.168.42.0/24 -j MASQUERADE
sudo sysctl net.ipv4.ip_forward=1
```
> Might get error: `Chain 'MASQUERADE' does not exist`.
> Setting iptables config to legacy can help `sudo update-alternatives --config iptables` select 1
2. Run
```
cargo run http://www.<domain-name>/ <tap-device-name>
```

### Code Structure 
1. `main.rs` - get inputs using `clap` parsing them and calling `http::get()`.
2. `dns.rs` - resolves dns using `trust_dns_client`.
3. `ethernet.rs` - provide mac address.
4. `http.rs` - builds a network interface and connects to the domain provided using `smoltcp`.

### DNS details
|Term|Definition|Code|
|-|-|-|
|Message| A message is a container for both requests to DNS servers and responses back to clients. Messages must contain a header, but other fields are not required. A `Message` struct represents this and includes several `Vec<T>` fields. These do not need to be wrapped in `Option` to represent missing values as their length can be 0.|[`trust_dns/crates/proto/src/op/message.rs`](https://github.com/bluejekyll/trust-dns/blob/05f9642f335070e00693d16817184752db1f62e2/crates/proto/src/op/message.rs#L65)|
|Message type| A message type identifies the message as query or as an answer. Queries can also be updates, which are functionality that out code ignores.|[`trust_dns/crates/proto/src/op/header.rs`](https://github.com/bluejekyll/trust-dns/blob/05f9642f335070e00693d16817184752db1f62e2/crates/proto/src/op/header.rs#L86)|
|Message ID| A number that is used for senders to link queries and answers.| `u16`|
|Resource record type| The resource record type refers to the DNS codes that you've probably encountered if you've ever configured a domain name. Of not is how `trust_dns` handles invalid codes. The `RecordType` enum contains an `Unknown(u16)` variant that can be used for codes that if doesn't understand.| [`trust_dns/crates/proto/src/rr/record_type.rs](https://github.com/bluejekyll/trust-dns/blob/95b2dee327ade007bf317ca98bbd3b24c0bdd096/crates/proto/src/rr/record_type.rs#L33)|
|Query| A `Query` struct holds the domain name and the record type that we're seeking the DNS details for. These traits also describe the DNS class and allow queries to distinguish between messages sent over the internet form other transport protocols.|  [`trust_dns/crates/proto/src/op/query.rs`](https://github.com/bluejekyll/trust-dns/blob/37d4a966db2dd67bc640bef0d6dcd5a37375d562/crates/proto/src/op/query.rs#L62)|
|Opcode| An `OpCode` enum is, in some sens, a subtype of `MessageType`. This is an extensibility mechanism that allows future functionality. for example, RFC 1035 defines the `Query` and `Status` opcodes but others were defined later. The `Notify` and `Update` opcodes are defined by RFC 1996 and RFC 2136, respectively.|[`trust_dns/crates/proto/src/op/opcode.rs`](https://github.com/bluejekyll/trust-dns/blob/37d4a966db2dd67bc640bef0d6dcd5a37375d562/crates/proto/src/op/op_code.rs#L33)|