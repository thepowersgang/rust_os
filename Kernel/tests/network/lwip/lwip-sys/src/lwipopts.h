#define NO_SYS	0
#define LWIP_SOCKET                (NO_SYS==0)
#define LWIP_NETCONN               (NO_SYS==0)
#define LWIP_NETIF_API             (NO_SYS==0)

#define LWIP_NUM_NETIF_CLIENT_DATA      1

#define	LWIP_IPV6_NUM_ADDRESSES	6	// NOTE: Something elsewhere is picking `6` so this is needed for ABI compat

#define MEM_ALIGNMENT	8
#define	LWIP_IPV4	1
#define LWIP_IPV6	1
#define	LWIP_DEBUG	1
#define	LWIP_STATS	0

#define LWIP_IPV6_ADDRESS_LIFETIMES	1

#define LWIP_NETIF_LINK_CALLBACK        0
#define LWIP_NETIF_STATUS_CALLBACK      0
#define LWIP_NETIF_EXT_STATUS_CALLBACK  0

//#define LWIP_DBG_MIN_LEVEL	LWIP_DBG_LEVEL_ALL
//#define PBUF_DEBUG	LWIP_DBG_ON

// suggested by an assertion failure
#define IPV6_FRAG_COPYHEADER	1