## Building docs

Uses nightly docs features on docs.rs. To build docs locally using nightly features:

```
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --no-deps
```

## Missing Features

- Support for multiple SDO servers
- Support for SYNC driven RPDO
- EDS generation
- Direct response in receive IRQ
    - Should support an optional immediate TX message queuing for fast turnaround, e.g. on LSS
      fastscan messages, regardless of process timing. It isn't required, but could be implemented
      on a lot of systems.
- Non-async API for zencan-client
- EMCY service
  - Send EMCY message on receiving PDO of invalid length
- zencandump needs to display COB-ID, e.g. for SdoRequest messages to distinguish nodes or servers
- integration test wrapper
  - DRY up the node process task creation
  - print returned errors with Display
- Add better validity checking on PDO configuration
  - Don't allow changing things while operating
- zencan-build should probably be exported and documented via zencan-node