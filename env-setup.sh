#!/bin/bash

set -e

# Update and install build dependencies
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential dkms git iw bc libpcap-dev libnl-3-dev libnl-genl-3-dev autoconf automake libtool pkg-config m4 make curl wget libssl-dev linux-tools-generic hwdata

# Install patched rtl8812au driver
sudo mkdir -p /opt/rtl8812au && sudo chown $USER:$USER /opt/rtl8812au
if [ ! -d /opt/rtl8812au/.git ]; then
    git clone https://github.com/svpcom/rtl8812au.git /opt/rtl8812au
else
    echo "/opt/rtl8812au already cloned, skipping git clone."
fi
pushd /opt/rtl8812au
sudo ./dkms-install.sh
popd

echo "blacklist rtw88_8812au" | sudo tee /etc/modprobe.d/blacklist-rtw88.conf
echo "blacklist rtw88_usb" | sudo tee -a /etc/modprobe.d/blacklist-rtw88.conf
echo "blacklist rtw88_core" | sudo tee -a /etc/modprobe.d/blacklist-rtw88.conf
sudo update-initramfs -u

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

echo "Setup complete"
