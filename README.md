# KeySync

KeySync is a simple keypress synchronization tool that allows users to share keypress events over the internet.
It operates in two modes: server and client.
The server broadcasts keypress events to all clients.
Clients configure key mappings to define what keys to emit, and how to map them, both for incoming and outgoing.

## How to run
```sh
# Start the server on port 1234:
keysync server
# Or, specify an address to bind to:
keysync server --bind-address 0.0.0.0:1234


# Start a client
keysync client -s 127.0.0.1:1234

```

## Configuration
The client is configurable through `config.hjson`, which is populated on the first run.

`incoming`: server -> local machine

`outgoing`: local machine -> server

`devices`: An allowlist of devices to monitor. Can be either a path to a device, or a regex.

Use `evscan` to identify keypress mappings, or look at [/usr/include/linux/input-event-codes.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)