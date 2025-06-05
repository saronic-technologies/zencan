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
