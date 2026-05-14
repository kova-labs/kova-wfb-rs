#!/bin/bash

if systemd-detect-virt | grep -q microsoft; then
    echo "Running inside Microsoft Hyper-V or WSL detected by systemd-detect-virt."
    sudo modprobe vhci-hcd

    DEFAULT_GW=$(ip route | awk '/default/ {print $3}' | head -n1)
    echo "Default gateway IP address: $DEFAULT_GW"

    USBIP_OUTPUT=$(usbip list --remote="$DEFAULT_GW")
    echo "$USBIP_OUTPUT" | grep -B2 "0bda:8812" || {
        echo "Device '0bda:8812 Realtek RTL8812AU' not found over usbip on $DEFAULT_GW."
        echo ""
        usbip list --remote="$DEFAULT_GW"
        echo ""
        echo "Do you have the USB WIFI adapter connected and bound on the Windows side?"
        exit 1
    }
    echo "Found '0bda:8812 Realtek RTL8812AU' device over usbip on $DEFAULT_GW."

    BUSID=$(echo "$USBIP_OUTPUT" | grep -B2 "0bda:8812" | grep -oP 'busid \K[\d-]+')
    if [ -z "$BUSID" ]; then
        echo "Could not extract BUSID for '0bda:8812 Realtek RTL8812AU' device."
        exit 1
    fi
    echo "Attaching USB device with BUSID: $BUSID from $DEFAULT_GW"
    sudo usbip attach --remote="$DEFAULT_GW" --busid="$BUSID"
else
    echo "Not running inside Microsoft Hyper-V or WSL detected by systemd-detect-virt."
fi
