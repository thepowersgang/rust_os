% Usermode API - Inter-Process Communication methods


Requirements
====
- Allow passing objects between processes
- Non-sharable channel handles
 - Only one process can hold a channel endpoint
- Needs to support passing strings of a non-trivial length

Ideas
====
- Synchronous message passing (basically RPC)
 - Simplest option, and can be made async (by having only one outstanding request per channel)
 - Also fits main usecase.
- Single-message async (each channel can have one outstanding message)
- Arbitary-length message queue


Semi-Syncronous Message Passing (RPC)
====================================

- Uni-directional channels (single message/object buffer)
 - The instigatior side (i.e. client) sends a message and optional object to the receiver
 - Receiver then sends a reply

- If a channel handle is in "waiting" state (i.e. has sent a message without receiving a reply). Then the channel is waitable.
 - Otherwise the wait terminates with an error?

- Messages are of a fixed maximum size (32/64 bytes?)

