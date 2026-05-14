#ifndef WFB_RS_H
#define WFB_RS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#define WFB_RS_OK 0
#define WFB_RS_ERR_NULL_PTR 1
#define WFB_RS_ERR_INVALID_ARGUMENT 2
#define WFB_RS_ERR_IO 3
#define WFB_RS_ERR_PCAP 4
#define WFB_RS_ERR_TIMEOUT 5
#define WFB_RS_ERR_INTERNAL 255

#define WFB_RS_ABI_VERSION 1

typedef struct wfb_tx_handle wfb_tx_handle;
typedef struct wfb_rx_handle wfb_rx_handle;

typedef struct wfb_tx_config {
  const char *iface;
  uint32_t channel_id;
  uint8_t frame_type;
  uint8_t mcs_index;
  uint8_t bandwidth;
} wfb_tx_config;

typedef struct wfb_rx_config {
  const char *iface;
  uint32_t channel_id;
  uint8_t ignore_self_injected;
  uint32_t ring_size;
} wfb_rx_config;

typedef struct wfb_rx_meta {
  uint32_t seq;
  uint8_t flags;
  uint16_t freq;
  uint8_t mcs_index;
  uint8_t bandwidth;
  uint8_t antenna[4];
  int8_t rssi[4];
  int8_t noise[4];
  uint8_t antenna_count;
  uint8_t truncated;
} wfb_rx_meta;

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint32_t wfb_rs_abi_version(void);

size_t wfb_rs_max_payload(void);

int32_t wfb_tx_open(const wfb_tx_config *cfg, wfb_tx_handle **out_handle);

int32_t wfb_tx_close(wfb_tx_handle *handle);

int32_t wfb_tx_send(wfb_tx_handle *handle, const uint8_t *payload, size_t payload_len, uint32_t seq);

int32_t wfb_rx_open(const wfb_rx_config *cfg, wfb_rx_handle **out_handle);

int32_t wfb_rx_close(wfb_rx_handle *handle);

int32_t wfb_rx_recv(wfb_rx_handle *handle,
                    uint8_t *out_buf,
                    size_t out_buf_len,
                    uint32_t timeout_ms,
                    size_t *out_len,
                    wfb_rx_meta *out_meta);

#ifdef __cplusplus
} // extern "C"
#endif // __cplusplus

#endif // WFB_RS_H
