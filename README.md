# KeySync

KeySync is a simple tool to broadcast keystrokes over a network.
It operates in two modes: server and client.
The server broadcasts keypress events to *all* clients.
Clients configure key mappings to define what keys to emit, and how to map them, both for incoming and outgoing.

It relies on evdev, and so is supported in Linux only.

## How to run
```sh
# Start the server on port 1234:
keysync server
# Or, specify an address to bind to:
keysync server --bind-address 0.0.0.0:1234


# Start a client
keysync client -s 127.0.0.1:1234
# If you have permission denied errors, you may need to put your user into
# the "input" group, or run with sudo.

```

## Configuration
The client is configurable through `config.yaml` (in your current working dir), and is populated on the first run.

`incoming`: server -> local machine

`outgoing`: local machine -> server

`devices`: An allowlist of devices to monitor. Can be either a path to a device, or a regex.

Use `evscan` to identify keypress mappings, or look at [/usr/include/linux/input-event-codes.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h)

Example:

```yaml
incoming:
  # When a client receives the escape key, the escape key is pressed.
  KEY_ESC: KEY_ESC
outgoing:
  # When a client presses the X key, the escape key is sent to the server.
  KEY_X: KEY_ESC
```

Note keys are sent back to the originating client as well.
