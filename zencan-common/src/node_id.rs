
/// An enum representing the node ID of a CANopen node. The node ID must be between 1 and 127 for
/// configured devices, with the special value of 255 used to represent an unconfigured device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeId {
    Unconfigured,
    Configured(NodeIdNum),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeIdNum(u8);
impl NodeIdNum {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if (value > 0 && value < 128) || value == 255 {
            Ok(NodeIdNum(value))
        } else {
            Err(InvalidNodeIdError)
        }
    }
}

impl NodeId {
    pub fn new(value: u8) -> Result<Self, InvalidNodeIdError> {
        if value == 255 {
            Ok(NodeId::Unconfigured)
        } else {
            NodeIdNum::new(value).map(NodeId::Configured)
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
            Ok(NodeId::Configured(NodeIdNum(value)))
        }
    }
}

impl From<NodeId> for u8 {
    fn from(value: NodeId) -> Self {
        match value {
            NodeId::Unconfigured => 255,
            NodeId::Configured(id) => id.0,
        }
    }
}