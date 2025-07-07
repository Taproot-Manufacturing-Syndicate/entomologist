#!/bin/bash 

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
BINFILE="${SCRIPT_DIR}/target/release/ent"
INSTALL_DIR="/usr/bin"

cargo build --release 
echo "copying ent to ${INSTALL_DIR}"
sudo cp $BINFILE $INSTALL_DIR
echo "ent installed to ${INSTALL_DIR}"
