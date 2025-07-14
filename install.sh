#!/bin/bash 

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
BINFILE="${SCRIPT_DIR}/target/release/ent"
ENTUI_BINFILE="${SCRIPT_DIR}/target/release/entui"
INSTALL_DIR="/usr/local/bin"

cargo build --release 
echo "copying ent + entui to ${INSTALL_DIR}"
sudo cp $BINFILE $ENTUI_BINFILE $INSTALL_DIR
echo "ent installed to ${INSTALL_DIR}"
