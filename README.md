## Missing Features

- Support for multiple SDO servers
- Support for SYNC RPDO
- Support for TPDOs
- Socketcan hosted demo node
- EDS generation
- Direct response in receive IRQ
    - Should support an optional immediate TX message queuing for fast turnaround, e.g. on LSS
      fastscan messages, regardless of process timing. It isn't required, but could be implemented
      on a lot of systems.
- Non-async API for zencan-client
- Implement periodic heartbeat