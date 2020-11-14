#!/bin/sh

# Uses state on Jeff's machine, not really scaleable

deploy_new_vm() {
  gcloud compute instances create "ptrack" \
    --image-project=arch-linux-gce \
    --image-family=arch \
    --machine-type=e2-small \
    --zone=us-east1-b \
    --network-tier=STANDARD \
    --boot-disk-size=32GB \
    --hostname="ptrack.jmcateer.pw"

}

set -e

cargo build --bin=ptrack-server --release --target=x86_64-unknown-linux-gnu
cargo build --bin=ptrack-agent --release --target=x86_64-pc-windows-gnu

gcloud compute ssh ptrack -- sudo mkdir -p /opt/ptrack/
gcloud compute scp target/x86_64-unknown-linux-gnu/release/ptrack-server ptrack:/opt/ptrack/
gcloud compute scp target/x86_64-pc-windows-gnu/release/ptrack-agent.exe ptrack:/opt/ptrack/
gcloud compute ssh ptrack -- sudo /opt/ptrack/ptrack-server install-and-run-systemd-service


