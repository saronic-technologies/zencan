# zencan-cli

Command line utilities for zencan

## zencandump

Monitor a bus, and print messages on stdout.

Usage: `zencandump vcan0`

## zencan-cli

An interactive shell for controlling a bus.

Usage: `zencan-cli vcan0`

Type `help` to get a list of available commands.

## Creating virtual socketcan adapters on linux

It's useful for testing to connect local nodes and the CLI tools over a virtual CAN bus.

```
sudo ip link add dev vcan0 type vcan
sudo ip link set up vcan0
```

## Loading configuration to devices

Often devices need to be configured by writing to various objects. For example, a generic device may
be configured to work in a specific system by setting up the correct PDO mappings. To support this, a TOML schema for node configuration is defined, and it can be loaded using the zencan-cli `load-config` command.

### Example node configuration file

This file configures TPDO0 to transmit the 16-bit value from object 0x2000sub0 using CAN ID 0x200, and it sets object 0x2001sub1 to a value of 42, presumably for reasons.

```toml
[tpdo.0]
enabled = true
cob = 0x200
transmission_type = 254
mappings = [
    { index=0x2000, sub=0, size=16 },
]

[[store]]
index = 0x2001
sub = 1
type = "u16"
value = 42
```

Once configured, the written values can be persisted using the save command, assuming the
application has implemented the storage callback.
