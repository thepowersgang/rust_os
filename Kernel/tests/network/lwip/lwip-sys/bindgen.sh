#!/bin/sh
set +eu
cd $(dirname $0)
ARGS=""
ARGS=${ARGS}" --allowlist-type err_enum_t"
# tcpip_* - Hosted/OS mode
ARGS=${ARGS}" --allowlist-function tcpip_.*"
# LWIP BSC socket functions
ARGS=${ARGS}" --allowlist-function lwip_.* --allowlist-type lwip_.*"
ARGS=${ARGS}" --allowlist-var SOCK_.*"
ARGS=${ARGS}" --allowlist-var AF_.*"
ARGS=${ARGS}" --allowlist-type sockaddr.*"
# netconn (native sockets)
ARGS=${ARGS}" --allowlist-function netconn_.*"
ARGS=${ARGS}" --allowlist-var NETCONN_FLAG_.*"
# low-level APIs
ARGS=${ARGS}" --allowlist-function netif_.* --allowlist-var NETIF_FLAG_.*"
ARGS=${ARGS}" --allowlist-function pbuf_.* --allowlist-var PBUF_.*"
ARGS=${ARGS}" --allowlist-function netbuf_.*"
ARGS=${ARGS}" --allowlist-function etharp_.*"
ARGS=${ARGS}" --allowlist-function ip[46]addr_.*"
bindgen template.h \
    ${ARGS} \
    --allowlist-function netifapi_.* \
    -- -I /usr/include/lwip/ > src/bindgen.rs