#!/bin/bash

set -Eeuo pipefail
set -v

cargo build --target armv7-unknown-linux-musleabihf
sshpass -p "$PASS" ssh -t root@odroid 'systemctl stop rut; rm -f /home/rut/db.sqlite3'
sshpass -p "$PASS" scp target/armv7-unknown-linux-musleabihf/debug/rut root@odroid:/usr/local/bin
sshpass -p "$PASS" scp index.html root@odroid:/home/rut
sshpass -p "$PASS" ssh -t root@odroid 'systemctl start rut'
