
/// An enum representing the node ID of a CANopen node. The node ID must be between 1 and 127 for
/// configured devices, with the special value of 255 used to represent an unconfigured device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeId {
    Unconfigured,
    Configured(ConfiguredId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfiguredId(u8);
impl ConfiguredId {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if (value > 0 && value < 128) || value == 255 {
            Ok(ConfiguredId(value))
        } else {
            Err(InvalidNodeIdError)
        }
    }

    pub fn raw(&self) -> u8 {
        self.0
    }
}

impl From<ConfiguredId> for u8 {
    fn from(value: ConfiguredId) -> Self {
        value.raw()
    }
}

impl NodeId {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if value == 255 {
            Ok(NodeId::Unconfigured)
        } else {
            ConfiguredId::new(value).map(NodeId::Configured)
        }
    }

    pub fn raw(&self) -> u8 {
        match self {
            NodeId::Unconfigured => 255,
            NodeId::Configured(node_id_num) => node_id_num.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNodeIdError;

impl core::fmt::Display for InvalidNodeIdError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Invalid node ID")
    }
}
impl core::error::Error for InvalidNodeIdError {}

impl TryFrom<u8> for NodeId {
    type Error = InvalidNodeIdError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 255 {
            Ok(NodeId::Unconfigured)
        } else {
            Ok(NodeId::Configured(ConfiguredId(value)))
        }
    }
}

impl From<NodeId> for u8 {
    fn from(value: NodeId) -> Self {
        value.raw()
    }
}