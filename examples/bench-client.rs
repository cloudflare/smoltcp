mod utils;

use log::{debug, info};
use std::collections::BTreeMap;
use std::os::unix::io::AsRawFd;
use std::str::FromStr;

use smoltcp::iface::{InterfaceBuilder, NeighborCache, Routes};
use smoltcp::phy::{wait as phy_wait, Device, DeviceCapabilities, Medium};
use smoltcp::socket::{SocketSet, TcpSocket, TcpSocketBuffer};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use smoltcp::Error;

fn main() {
    utils::setup_logging("");

    let (mut opts, mut free) = utils::create_options();
    utils::add_tuntap_options(&mut opts, &mut free);
    utils::add_middleware_options(&mut opts, &mut free);
    free.push("ADDRESS");
    free.push("PORT");

    let mut matches = utils::parse_options(&opts, free);
    let device = utils::parse_tuntap_options(&mut matches);

    let fd = device.as_raw_fd();
    let device = utils::parse_middleware_options(&mut matches, device, /*loopback=*/ false);
    let device = YieldDevice {
        device,
        yield_count: 2,
    };
    let address = IpAddress::from_str(&matches.free[0]).expect("invalid address format");
    let port = u16::from_str(&matches.free[1]).expect("invalid port format");

    let neighbor_cache = NeighborCache::new(BTreeMap::new());

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024 * 1024]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024 * 1024]);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

    let ethernet_addr = EthernetAddress([0x02, 0x00, 0x00, 0x00, 0x00, 0x02]);
    let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24)];
    let default_v4_gw = Ipv4Address::new(192, 168, 69, 100);
    let mut routes_storage = [None; 1];
    let mut routes = Routes::new(&mut routes_storage[..]);
    routes.add_default_ipv4_route(default_v4_gw).unwrap();

    let medium = device.capabilities().medium;
    let mut builder = InterfaceBuilder::new(device)
        .ip_addrs(ip_addrs)
        .routes(routes);
    if medium == Medium::Ethernet {
        builder = builder
            .hardware_addr(ethernet_addr.into())
            .neighbor_cache(neighbor_cache);
    }
    let mut iface = builder.finalize();

    let mut sockets = SocketSet::new(vec![]);
    let tcp_handle = sockets.add(tcp_socket);

    {
        let mut socket = sockets.get::<TcpSocket>(tcp_handle);
        socket.connect((address, port), 49500).unwrap();
    }

    let mut tcp_active = false;
    let mut request = Some(b"GET /giant-file HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n");
    let mut bytes_received = 0;
    'outer: loop {
        let timestamp = Instant::now();
        match iface.poll(&mut sockets, timestamp) {
            Ok(_) => {}
            Err(e) => {
                debug!("poll error: {}", e);
            }
        }

        {
            let mut socket = sockets.get::<TcpSocket>(tcp_handle);
            if socket.is_active() && !tcp_active {
                debug!("connected");
            } else if !socket.is_active() && tcp_active {
                debug!("disconnected");
                break;
            }
            tcp_active = socket.is_active();

            if request.is_some() && socket.can_send() {
                socket.send_slice(request.take().unwrap()).unwrap();
            }

            if socket.may_recv() {
                loop {
                    match socket
                        .recv(|data| {
                            if !data.is_empty() {
                                debug!("recv {} bytes of data", data.len());
                            }
                            bytes_received += data.len();
                            (data.len(), data.len())
                        }) {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(Error::Finished) => {
                            debug!("close");
                            socket.close();
                            break 'outer;
                        }
                        Err(e) => {
                            info!("socket error: {}", e);
                            break 'outer;
                        }
                    }
                }
            }
        }

        phy_wait(fd, iface.poll_delay(&sockets, timestamp)).expect("wait error");
    }
    info!("received {} bytes total", bytes_received);
}

struct YieldDevice<D> {
    device: D,
    yield_count: u8,
}

impl<'a, D> Device<'a> for YieldDevice<D>
where
    D: for<'b> Device<'b>,
{
    type RxToken = <D as Device<'a>>::RxToken;
    type TxToken = <D as Device<'a>>::TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        if self.yield_count > 0 {
            if let Some((r, t)) = self.device.receive() {
                self.yield_count -= 1;
                return Some((r, t));
            }
        }
        self.yield_count = 2;
        None
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        self.device.transmit()
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.device.capabilities()
    }
}
