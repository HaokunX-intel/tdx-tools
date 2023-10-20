#!/bin/bash

#cargo build --features DEBUG
#scp target/debug/ek-tool tdx@10.1.63.48:/home/tdx

cargo build --release
#scp target/release/ek-tool tdx@10.1.63.48:/home/tdx
