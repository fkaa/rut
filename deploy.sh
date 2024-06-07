#!/bin/bash

set -Eeuo pipefail
set -v

# cargo build --target armv7-unknown-linux-musleabihf
# sshpass -p "$PASS" ssh -t tmtu@rascalberry 'systemctl stop rut'
# sshpass -p "$PASS" scp target/armv7-unknown-linux-musleabihf/debug/rut tmtu@rascalberry:/usr/local/bin
sshpass -p "$PASS" scp index.html tmtu@rascalberry:/home/tmtu
sshpass -p "$PASS" ssh -t tmtu@rascalberry 'sudo mv /home/tmtu/index.html /var/lib/rut/index.html'
# sshpass -p "$PASS" ssh -t tmtu@rascalberry 'systemctl start rut'
