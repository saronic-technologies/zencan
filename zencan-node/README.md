# zencan-node

Crate for implementing a zencan device. Usually used in an embedded `no_std` context, but also can be used elsewhere.

## Usage

### Create a device config

First create a TOML file to define the properties of your node. This include some options, like the device name and identity information, as well as a list of application specific object to be created. All nodes get a set of standard objects, in addition to the custom ones defined in the device config.

#### Sample device config

```toml
device_name = "can-io"

[identity]
vendor_id = 0xCAFE
product_code = 32
revision_number = 1

[[pdos]]
num_tpdos = 4
num_rpdos = 4

# Create an array object to store the four u16 raw analog readings
# It is read-only, so it can only be updated by the application
# It can be mapped to transmit PDOs for sending data to the bus
[[objects]]
index = 0x2000
parameter_name = "Raw Analog Input"
object_type = "array"
data_type = "uint16"
access_type = "ro"
array_size = 4
default_value = [0, 0, 0, 0]
pdo_mapping = "tpdo"

# An object for storing the scaled analog values
[[objects]]
index = 0x2001
parameter_name = "Scaled Analog Input"
object_type = "array"
data_type = "uint16"
access_type = "ro"
array_size = 4
default_value = [0, 0, 0, 0]
pdo_mapping = "tpdo"


# A configuration object for controlling the frequency of analog readings
[[objects]]
index = 0x2100
parameter_name = "Analog Read Period (ms)"
object_type = "var"
data_type = "uint32"
access_type = "rw"
default_value = 10


# A configuration object which can be used to adjust the linear offset and scale transform used to
# calculate the value in 0x2001
[[objects]]
index = 0x2101
parameter_name = "Analog Scale Config"
object_type = "record"
[[objects.subs]]
sub_index = 1
parameter_name = "Scale Numerator"
data_type = "uint16"
access_type = "rw"
default_value = 1
[[objects.subs]]
sub_index = 2
parameter_name = "Scale Denominator"
data_type = "uint16"
access_type = "rw"
default_value = 1
[[objects.subs]]
sub_index = 3
parameter_name = "Offset"
data_type = "uint16"
access_type = "rw"
default_value = 0
```

### Add a zencan-build as a dev-dependency

```
cargo add --build zencan-build
```

### Build the generated code in `build.rs`

In your `build.rs` script, add code like this:

```rust
fn main() {
    if let Err(e) = zencan_build::build_node_from_device_config("ZENCAN_CONFIG", "zencan_config.toml") {
        eprintln!("Failed to parse zencan_config.toml: {}", e.to_string());
        std::process::exit(-1);
    }
}
```

### Include the generated code in your application

For example, in main.rs:

```rust
mod zencan {
    zencan_node::include_modules!(ZENCAN_CONFIG);
}
```

### Instantiate a node

```rust
    let node = Node::new(
        NodeId::Unconfigured,
        &zencan::NODE_MBOX,
        &zencan::NODE_STATE,
        &zencan::OD_TABLE,
    );

    // Use the UID register or some other method to set a unique 32-bit serial number
    let serial_number: u32 = get_serial();
    zencan::OBJECT1018.set_serial(serial_number);
```

### Feed it incoming messages, and poll process

Since Zencan doesn't know what your CAN interface looks like, you have to do some plumbing here to wire it up to how you send and receive messages.

Received messages can be passed in using the `NODE_MBOX` struct, e.g.:

```rust
zencan::NODE_MBOX.store_message(msg)?;
```

The node `process()` method must be called from time to time. It isn't critical how fast, but your node's response time depend on it. The NODE_MBOX also provides the `set_process_notify_callback` method, to register a callback which will be called whenever there is new information to be processed so that the `process()` call can be accelerated.

```rust
/// A task for running the CAN node processing periodically, or when triggered by the CAN receive
/// interrupt to run immediately
async fn can_task(
    mut node: Node<'static>,
    mut can_tx: fdcan::Tx<FdCan1, NormalOperationMode>,
) -> Infallible {
    let epoch = lilos::time::TickTime::now();
    loop {
        lilos::time::with_timeout(Duration::from_millis(10), CAN_NOTIFY.until_next()).await;
        let time_us = epoch.elapsed().0 * 1000;
        // Process is called with the current time, so that it can execute periodic tasks, and a
        // callback for transmitting messages.
        node.process(time_us, &mut |msg| {
            // Convert between zencan and fdcan frame types
            let id: fdcan::id::Id = match msg.id() {
                zencan_node::common::messages::CanId::Extended(id) => {
                    fdcan::id::ExtendedId::new(id).unwrap().into()
                }
                zencan_node::common::messages::CanId::Std(id) => {
                    fdcan::id::StandardId::new(id).unwrap().into()
                }
            };
            let header = fdcan::frame::TxFrameHeader {
                len: msg.dlc,
                frame_format: fdcan::frame::FrameFormat::Standard,
                id,
                bit_rate_switching: false,
                marker: None,
            };
            can_tx.transmit(header, msg.data()).ok();
        });
    }
}
```

### Socketcan Example

You can also run a node on linux, in socketcan. Useful for testing with a virtual can adapter. See
[this example](../examples/socketcan_node/) for a full implementation.