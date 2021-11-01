#!/bin/bash

NGINX_PATH=$PWD/bench-support
STATIC_FILE_PATH=$NGINX_PATH/html

# Cleanup any left over runs
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf -s stop 2>/dev/null || true
sudo ip tuntap del smoltcp0 mode tun 2>/dev/null || true

# Setup
sudo ip tuntap add name smoltcp0 mode tun
sudo ip addr add 192.168.69.1/24 dev smoltcp0
sudo ip link set up dev smoltcp0
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf
if [ ! -f $STATIC_FILE_PATH/giant-file ]; then
    dd if=/dev/urandom of=$STATIC_FILE_PATH/giant-file bs=1024 count=10240
fi
cargo build --release --example bench-client

function run_smoltcp {
    echo "via smoltcp:"
    time RUST_LOG=debug target/release/examples/bench-client --tun smoltcp0 192.168.69.1 21080
}

function run_tests {
    echo "via curl:"
    time curl -so /dev/null http://127.0.0.1:21080/giant-file
    run_smoltcp
}

# Bench
echo "== BASELINE TESTS =="
run_tests

# Bench with loss
sudo tc qdisc add dev smoltcp0 root netem loss 0.5% delay 20ms
echo "== 1% LOSS TESTS =="
run_smoltcp

# Cleanup
/usr/sbin/nginx -p $NGINX_PATH -c nginx.conf -s stop 2>/dev/null
sudo ip tuntap del smoltcp0 mode tun
