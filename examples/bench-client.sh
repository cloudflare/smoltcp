#!/bin/bash

NGINX_PATH=$PWD/bench-support
STATIC_FILE_PATH=$NGINX_PATH/html

# Cleanup any left over runs
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf -s stop 2>/dev/null || true
sudo ip tuntap del smoltcp0 mode tun 2>/dev/null || true
sudo ip netns del smoltcp-bench

# Setup
sudo ip tuntap add name smoltcp0 mode tun
sudo ip addr add 192.168.69.1/24 dev smoltcp0
sudo ip link set up dev smoltcp0
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf
if [ ! -f $STATIC_FILE_PATH/giant-file ]; then
    dd if=/dev/urandom of=$STATIC_FILE_PATH/giant-file bs=1024 count=10240
fi
cargo build --release --example bench-client
# Baseline environment with curl
sudo ip netns add smoltcp-bench
sudo ip link add veth-delay type veth peer name veth-delay2 netns smoltcp-bench
sudo ip addr add 192.0.2.1/24 dev veth-delay
sudo ip link set up veth-delay
sudo ip -n smoltcp-bench addr add 192.0.2.2/24 dev veth-delay2
sudo ip -n smoltcp-bench link set up veth-delay2

function run_tests {
    echo "via curl:"
    time sudo ip netns exec smoltcp-bench curl -o /dev/null http://192.0.2.1:21080/giant-file
    echo "via smoltcp:"
    time RUST_LOG=debug target/release/examples/bench-client --tun smoltcp0 192.168.69.1 21080
}

# Bench
echo "== BASELINE TESTS =="
run_tests

# Bench with loss
sudo tc qdisc add dev smoltcp0 root netem loss 0.5% delay 20ms
sudo tc qdisc add dev veth-delay root netem loss 0.5% delay 20ms
echo "== 1% LOSS TESTS =="
run_tests

# Cleanup
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf -s stop 2>/dev/null
sudo ip tuntap del smoltcp0 mode tun
sudo ip netns del smoltcp-bench
