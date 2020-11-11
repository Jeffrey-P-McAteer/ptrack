#!/bin/sh

# Uses state on Jeff's machine, not really scaleable

set -e

cargo build --bin=ptrack-server --release
cargo build --bin=ptrack-agent --release --target=x86_64-pc-windows-gnu

gcloud compute scp target/release/ptrack-server meili-svr:/opt/ptrack/
gcloud compute scp target/x86_64-pc-windows-gnu/release/ptrack-agent.exe meili-svr:/opt/ptrack/
gcloud compute ssh meili-svr -- sudo /opt/ptrack/ptrack-server


