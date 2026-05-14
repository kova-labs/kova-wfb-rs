#include <stdint.h>
#include <stdio.h>

#include "wfb_rs.h"

int main(void) {
    uint32_t abi = wfb_rs_abi_version();
    size_t max_payload = wfb_rs_max_payload();

    printf("wfb_rs ABI v%u, max payload=%zu\n", abi, max_payload);

    /* Link-time smoke test for C ABI declarations and symbols. */
    wfb_tx_handle *tx = NULL;
    wfb_tx_config tx_cfg = {
        .iface = "wlan0",
        .channel_id = 1,
        .frame_type = 0x08,
        .mcs_index = 1,
        .bandwidth = 20,
    };
    (void)wfb_tx_open(&tx_cfg, &tx);

    return 0;
}
