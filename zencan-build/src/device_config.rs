//! Device configuration support
//!
//! Includes code for reading device config files and for implementing the "standard" objects in a
//! node.

use serde::{de::Error, Deserialize};
use zencan_common::objects::{AccessType, ObjectCode};

use crate::errors::*;
use snafu::ResultExt as _;

fn mandatory_objects(config: &DeviceConfig) -> Vec<ObjectDefinition> {
    vec![
        ObjectDefinition {
            index: 0x1000,
            parameter_name: "Device Type".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::UInt32,
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::Integer(0x00000000)),
                pdo_mapping: PdoMapping::None,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x1001,
            parameter_name: "Error Register".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::UInt8,
                access_type: AccessType::Ro.into(),
                default_value: Some(DefaultValue::Integer(0x00000000)),
                pdo_mapping: PdoMapping::None,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x1008,
            parameter_name: "Manufacturer Device Name".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::VisibleString(config.device_name.len()),
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::String(config.device_name.clone())),
                pdo_mapping: PdoMapping::None,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x1009,
            parameter_name: "Manufacturer Hardware Version".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::VisibleString(config.hardware_version.len()),
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::String(config.hardware_version.clone())),
                pdo_mapping: PdoMapping::None,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x100A,
            parameter_name: "Manufacturer Software Version".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::VisibleString(config.software_version.len()),
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::String(config.software_version.clone())),
                pdo_mapping: PdoMapping::None,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x1010,
            parameter_name: "Object Save Command".to_string(),
            application_callback: true,
            object: Object::Array(ArrayDefinition {
                data_type: DataType::UInt32,
                access_type: AccessType::Rw.into(),
                array_size: 1,
                persist: false,
                ..Default::default()
            }),
        },
        ObjectDefinition {
            index: 0x1017,
            parameter_name: "Heartbeat Producer Time (ms)".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::UInt16,
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::Integer(config.heartbeat_period as i64)),
                pdo_mapping: PdoMapping::None,
                persist: false,
            }),
        },
        ObjectDefinition {
            index: 0x1018,
            parameter_name: "Identity".to_string(),
            application_callback: false,
            object: Object::Record(RecordDefinition {
                subs: vec![
                    SubDefinition {
                        sub_index: 1,
                        parameter_name: "Vendor ID".to_string(),
                        field_name: Some("vendor_id".into()),
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: Some(DefaultValue::Integer(
                            config.identity.vendor_id as i64,
                        )),
                        pdo_mapping: PdoMapping::None,
                        ..Default::default()
                    },
                    SubDefinition {
                        sub_index: 2,
                        parameter_name: "Product Code".to_string(),
                        field_name: Some("product_code".into()),
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: Some(DefaultValue::Integer(
                            config.identity.product_code as i64,
                        )),
                        pdo_mapping: PdoMapping::None,
                        ..Default::default()
                    },
                    SubDefinition {
                        sub_index: 3,
                        parameter_name: "Revision Number".to_string(),
                        field_name: Some("revision".into()),
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: Some(DefaultValue::Integer(
                            config.identity.revision_number as i64,
                        )),
                        pdo_mapping: PdoMapping::None,
                        ..Default::default()
                    },
                    SubDefinition {
                        sub_index: 4,
                        parameter_name: "Serial Number".to_string(),
                        field_name: Some("serial".into()),
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: Some(DefaultValue::Integer(0)),
                        pdo_mapping: PdoMapping::None,
                        ..Default::default()
                    },
                ],
            }),
        },
    ]
}

fn pdo_objects(num_rpdo: usize, num_tpdo: usize) -> Vec<ObjectDefinition> {
    let mut objects = Vec::new();

    fn add_objects(objects: &mut Vec<ObjectDefinition>, i: usize, tx: bool) {
        let pdo_type = if tx { "TPDO" } else { "RPDO" };
        let comm_index = if tx { 0x1800 } else { 0x1400 };
        let mapping_index = if tx { 0x1A00 } else { 0x1600 };

        objects.push(ObjectDefinition {
            index: comm_index + i as u16,
            parameter_name: format!("{}{} Communication Parameter", pdo_type, i),
            application_callback: true,
            object: Object::Record(RecordDefinition {
                subs: vec![
                    SubDefinition {
                        sub_index: 1,
                        parameter_name: format!("COB-ID for {}{}", pdo_type, i),
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Rw.into(),
                        default_value: None,
                        pdo_mapping: PdoMapping::None,
                        persist: true,
                    },
                    SubDefinition {
                        sub_index: 2,
                        parameter_name: format!("Transmission type for {}{}", pdo_type, i),
                        field_name: None,
                        data_type: DataType::UInt8,
                        access_type: AccessType::Rw.into(),
                        default_value: None,
                        pdo_mapping: PdoMapping::None,
                        persist: true,
                    },
                ],
            }),
        });

        let mut mapping_subs = vec![SubDefinition {
            sub_index: 0,
            parameter_name: "Valid Mappings".to_string(),
            field_name: None,
            data_type: DataType::UInt8,
            access_type: AccessType::Rw.into(),
            default_value: Some(DefaultValue::Integer(0)),
            pdo_mapping: PdoMapping::None,
            persist: true,
        }];
        for sub in 1..65 {
            mapping_subs.push(SubDefinition {
                sub_index: sub,
                parameter_name: format!("{}{} Mapping App Object {}", pdo_type, i, sub),
                field_name: None,
                data_type: DataType::UInt32,
                access_type: AccessType::Rw.into(),
                default_value: None,
                pdo_mapping: PdoMapping::None,
                persist: true,
            });
        }

        objects.push(ObjectDefinition {
            index: mapping_index + i as u16,
            parameter_name: format!("{}{} Mapping Parameters", pdo_type, i),
            application_callback: true,
            object: Object::Record(RecordDefinition { subs: mapping_subs }),
        });
    }
    for i in 0..num_rpdo {
        add_objects(&mut objects, i, false);
    }
    for i in 0..num_tpdo {
        add_objects(&mut objects, i, true);
    }
    objects
}

fn default_num_rpdo() -> u8 {
    4
}
fn default_num_tpdo() -> u8 {
    4
}

/// Configuration options for PDOs
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct PdoConfig {
    #[serde(default = "default_num_rpdo")]
    /// The number of TX PDO slots available in the device. Defaults to 4.
    pub num_tpdo: u8,
    #[serde(default = "default_num_tpdo")]
    /// The number of RX PDO slots available in the device. Defaults to 4.
    pub num_rpdo: u8,
}

impl Default for PdoConfig {
    fn default() -> Self {
        Self {
            num_tpdo: default_num_tpdo(),
            num_rpdo: default_num_rpdo(),
        }
    }
}

/// The device identity is a unique 128-bit number used for addressing the device on the bus
///
/// The configures the three hardcoded components of the identity. The serial number component of
/// the identity must be set by the application to be unique, e.g. based on a value programmed into
/// non-volatile memory or from a UID register on the MCU.
#[derive(Deserialize, Debug, Default, Clone, Copy)]
#[serde(deny_unknown_fields)]
pub struct IdentityConfig {
    /// The 32-bit vendor ID for this device
    pub vendor_id: u32,
    /// The 32-bit product code for this device
    pub product_code: u32,
    /// The 32-bit revision number for this device
    pub revision_number: u32,
}

/// Enum indicating what PDO mappings a sub object supports
#[derive(Deserialize, Debug, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum PdoMapping {
    /// Cannot be mapped to any PDOs
    #[default]
    None,
    /// Can be mapped only to TPDOs
    Tpdo,
    /// Can be mapped only to RPDOs
    Rpdo,
    /// Can be mapped to any PDO
    Both,
}

impl PdoMapping {
    /// Can be mapped to a TPDO
    pub fn supports_tpdo(&self) -> bool {
        matches!(self, PdoMapping::Tpdo | PdoMapping::Both)
    }

    /// Can be mapped to an RPDO
    pub fn supports_rpdo(&self) -> bool {
        matches!(self, PdoMapping::Rpdo | PdoMapping::Both)
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
/// Device configuration structure
pub struct DeviceConfig {
    /// The name describing the type of device (e.g. a model)
    pub device_name: String,

    /// A version describing the hardware
    #[serde(default)]
    pub hardware_version: String,
    /// A version describing the software
    #[serde(default)]
    pub software_version: String,

    /// The period at which to transmit heartbeat messages in milliseconds
    #[serde(default)]
    pub heartbeat_period: u16,

    /// Configures the identity object on the device
    pub identity: IdentityConfig,

    /// Configure PDO settings
    #[serde(default)]
    pub pdos: PdoConfig,

    /// A list of application specific objects to define on the device
    #[serde(default)]
    pub objects: Vec<ObjectDefinition>,
}

/// Defines a sub-object in a record
#[derive(Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct SubDefinition {
    /// Sub index for the sub-object being defined
    pub sub_index: u8,
    /// A human readable name for the value stored in this sub-object
    #[serde(default)]
    pub parameter_name: String,
    /// Used to name the struct field associated with this sub object
    ///
    /// This is only applicable to record objects. If no name is provided, the default field name
    /// will be `sub[index]`, where index is the uppercase hex representation of the sub index
    #[serde(default)]
    pub field_name: Option<String>,
    /// The data type of the sub object
    pub data_type: DataType,
    /// Access permissions for the sub object
    #[serde(default)]
    pub access_type: AccessTypeDeser,
    /// The default value for the sub object
    #[serde(default)]
    pub default_value: Option<DefaultValue>,
    /// Indicates whether this sub object can be mapped to PDOs
    #[serde(default)]
    pub pdo_mapping: PdoMapping,
    /// Indicates if this sub object should be saved when the save command is sent
    #[serde(default)]
    pub persist: bool,
}

/// An enum to represent object default values
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DefaultValue {
    /// A default value for integer fields
    Integer(i64),
    /// A default value for float fields
    Float(f64),
    /// A default value for string fields
    String(String),
}

/// An enum representing the different types of objects which can be defined in a device config
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "object_type", rename_all = "lowercase")]
pub enum Object {
    /// A var object is just a single value
    Var(VarDefinition),
    /// An array object is an array of values, all with the same type
    Array(ArrayDefinition),
    /// A record is a collection of sub objects all with different types
    Record(RecordDefinition),
    /// A domain is a chunk of bytes which can be accessed via the object
    Domain(DomainDefinition),
}

/// Descriptor for a var object
#[derive(Default, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VarDefinition {
    /// Indicates the type of data stored in the object
    pub data_type: DataType,
    /// Indicates how this object can be accessed
    pub access_type: AccessTypeDeser,
    /// The default value for this object
    pub default_value: Option<DefaultValue>,
    /// Determines which if type of PDO this object can me mapped to
    #[serde(default)]
    pub pdo_mapping: PdoMapping,
    /// Indicates that this object should be saved
    #[serde(default)]
    pub persist: bool,
}

/// Descriptor for an array object
#[derive(Default, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArrayDefinition {
    /// The datatype of array fields
    pub data_type: DataType,
    /// Access type for all array fields
    pub access_type: AccessTypeDeser,
    /// The number of elements in the array
    pub array_size: usize,
    /// Default values for all array fields
    pub default_value: Option<Vec<DefaultValue>>,
    #[serde(default)]
    /// Whether fields in this array can be mapped to PDOs
    pub pdo_mapping: PdoMapping,
    #[serde(default)]
    /// Whether this array should be saved to flash on command
    pub persist: bool,
}

/// Descriptor for a record object
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RecordDefinition {
    /// The sub object definitions for this record object
    pub subs: Vec<SubDefinition>,
}

/// Descriptor for a domain object
///
/// Not yet implemented
#[derive(Clone, Copy, Deserialize, Debug)]
pub struct DomainDefinition {}

/// Descriptor for an object in the object dictionary
#[derive(Deserialize, Debug, Clone)]
pub struct ObjectDefinition {
    /// The index of the object
    pub index: u16,
    /// A human readable name to describe the contents of the object
    #[serde(default)]
    pub parameter_name: String,
    #[serde(default)]
    /// If true, this object is implemented by an application callback, and no storage will be
    /// allocated for it in the object dictionary.
    pub application_callback: bool,
    /// The descriptor for the object
    #[serde(flatten)]
    pub object: Object,
}

impl ObjectDefinition {
    /// Get the object code specifying the type of this object
    pub fn object_code(&self) -> ObjectCode {
        match self.object {
            Object::Var(_) => ObjectCode::Var,
            Object::Array(_) => ObjectCode::Array,
            Object::Record(_) => ObjectCode::Record,
            Object::Domain(_) => ObjectCode::Domain,
        }
    }
}

impl DeviceConfig {
    /// Try to read a device config from a file
    pub fn load(config_path: impl AsRef<std::path::Path>) -> Result<Self, CompileError> {
        let config_str = std::fs::read_to_string(&config_path).context(IoSnafu)?;
        let mut config: DeviceConfig = toml::from_str(&config_str).context(ParseTomlSnafu {
            message: format!("Error parsing {}", config_path.as_ref().display()),
        })?;

        // Add mandatory objects to the config
        config.objects.extend(mandatory_objects(&config));

        config.objects.extend(pdo_objects(
            config.pdos.num_rpdo as usize,
            config.pdos.num_tpdo as usize,
        ));

        Ok(config)
    }

    /// Try to read a config from a &str
    pub fn load_from_str(config_str: &str) -> Result<Self, CompileError> {
        let mut config: DeviceConfig = toml::from_str(config_str).context(ParseTomlSnafu {
            message: "Error parsing device config string".to_string(),
        })?;

        // Add mandatory objects to the config
        config.objects.extend(mandatory_objects(&config));

        config.objects.extend(pdo_objects(
            config.pdos.num_rpdo as usize,
            config.pdos.num_tpdo as usize,
        ));

        Ok(config)
    }
}

/// A newtype for ObjectCode to implement deserialization so we can use it in toml files
#[derive(Clone, Copy, Debug, Default)]
pub struct ObjectCodeDeser(pub ObjectCode);
impl<'de> serde::Deserialize<'de> for ObjectCodeDeser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // First try parsing it as a u8, if that fails, try looking for the string representation
        match s.parse::<u8>() {
            Ok(int_code) => match ObjectCode::try_from(int_code) {
                Ok(obj_code) => Ok(ObjectCodeDeser(obj_code)),
                Err(_) => Err(D::Error::custom(format!(
                    "Invalid object code: {}",
                    int_code
                ))),
            },
            Err(_) => match s.to_lowercase().as_str() {
                "null" => Ok(ObjectCodeDeser(ObjectCode::Null)),
                "domain" => Ok(ObjectCodeDeser(ObjectCode::Domain)),
                "deftype" => Ok(ObjectCodeDeser(ObjectCode::DefType)),
                "defstruct" => Ok(ObjectCodeDeser(ObjectCode::DefStruct)),
                "var" => Ok(ObjectCodeDeser(ObjectCode::Var)),
                "array" => Ok(ObjectCodeDeser(ObjectCode::Array)),
                "record" => Ok(ObjectCodeDeser(ObjectCode::Record)),
                _ => Err(D::Error::custom(format!("Invalid object code: {}", s))),
            },
        }
    }
}
impl From<ObjectCode> for ObjectCodeDeser {
    fn from(obj_code: ObjectCode) -> Self {
        ObjectCodeDeser(obj_code)
    }
}

/// A newtype on AccessType to implement serialization
#[derive(Clone, Copy, Debug, Default)]
pub struct AccessTypeDeser(pub AccessType);
impl<'de> serde::Deserialize<'de> for AccessTypeDeser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "ro" => Ok(AccessTypeDeser(AccessType::Ro)),
            "rw" => Ok(AccessTypeDeser(AccessType::Rw)),
            "wo" => Ok(AccessTypeDeser(AccessType::Wo)),
            "const" => Ok(AccessTypeDeser(AccessType::Const)),
            _ => Err(D::Error::custom(format!(
                "Invalid access type: {} (allowed: 'ro', 'rw', 'wo', or 'const')",
                s
            ))),
        }
    }
}
impl From<AccessType> for AccessTypeDeser {
    fn from(access_type: AccessType) -> Self {
        AccessTypeDeser(access_type)
    }
}

/// A type to represent data_type fields in a device config
///
/// This is similar, but slightly different from the DataType defined in `zencan_common`
#[derive(Clone, Copy, Debug, Default)]
#[allow(missing_docs)]
pub enum DataType {
    Boolean,
    Int8,
    Int16,
    Int32,
    #[default]
    UInt8,
    UInt16,
    UInt32,
    Real32,
    VisibleString(usize),
    OctetString(usize),
    UnicodeString(usize),
    TimeOfDay,
    TimeDifference,
    Domain,
}

impl DataType {
    /// Returns true if the type is one of the stringy types
    pub fn is_str(&self) -> bool {
        matches!(
            self,
            DataType::VisibleString(_) | DataType::OctetString(_) | DataType::UnicodeString(_)
        )
    }

    /// Get the storage size of the data type
    pub fn size(&self) -> usize {
        match self {
            DataType::Boolean => 1,
            DataType::Int8 => 1,
            DataType::Int16 => 2,
            DataType::Int32 => 4,
            DataType::UInt8 => 1,
            DataType::UInt16 => 2,
            DataType::UInt32 => 4,
            DataType::Real32 => 4,
            DataType::VisibleString(size) => *size,
            DataType::OctetString(size) => *size,
            DataType::UnicodeString(size) => *size,
            DataType::TimeOfDay => 4,
            DataType::TimeDifference => 4,
            DataType::Domain => 0, // Domain size is variable
        }
    }
}

impl<'de> serde::Deserialize<'de> for DataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let re_visiblestring = regex::Regex::new(r"^visiblestring\((\d+)\)$").unwrap();
        let re_octetstring = regex::Regex::new(r"^octetstring\((\d+)\)$").unwrap();
        let re_unicodestring = regex::Regex::new(r"^unicodestring\((\d+)\)$").unwrap();

        let s = String::deserialize(deserializer)?.to_lowercase();
        if s == "boolean" {
            Ok(DataType::Boolean)
        } else if s == "int8" {
            return Ok(DataType::Int8);
        } else if s == "int16" {
            return Ok(DataType::Int16);
        } else if s == "int32" {
            return Ok(DataType::Int32);
        } else if s == "uint8" {
            return Ok(DataType::UInt8);
        } else if s == "uint16" {
            return Ok(DataType::UInt16);
        } else if s == "uint32" {
            return Ok(DataType::UInt32);
        } else if s == "real32" {
            return Ok(DataType::Real32);
        } else if let Some(caps) = re_visiblestring.captures(&s) {
            let size: usize = caps[1].parse().map_err(|_| {
                D::Error::custom(format!("Invalid size for VisibleString: {}", &caps[1]))
            })?;
            return Ok(DataType::VisibleString(size));
        } else if let Some(caps) = re_octetstring.captures(&s) {
            let size: usize = caps[1].parse().map_err(|_| {
                D::Error::custom(format!("Invalid size for OctetString: {}", &caps[1]))
            })?;
            return Ok(DataType::OctetString(size));
        } else if let Some(caps) = re_unicodestring.captures(&s) {
            let size: usize = caps[1].parse().map_err(|_| {
                D::Error::custom(format!("Invalid size for UnicodeString: {}", &caps[1]))
            })?;
            return Ok(DataType::UnicodeString(size));
        } else if s == "timeofday" {
            return Ok(DataType::TimeOfDay);
        } else if s == "timedifference" {
            return Ok(DataType::TimeDifference);
        } else if s == "domain" {
            return Ok(DataType::Domain);
        } else {
            return Err(D::Error::custom(format!("Invalid data type: {}", s)));
        }
    }
}

// #[derive(Debug, Default, Clone)]
// ///A newtype to implement deserialization for DataType from another crate
// pub struct DataTypeDeser(pub DataType);
// impl<'de> serde::Deserialize<'de> for DataTypeDeser {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         let s = String::deserialize(deserializer)?;
//         // First try parsing it as an integer, if that fails, try looking for the string representation
//         match s.parse::<u16>() {
//             Ok(int_code) => match DataType::try_from(int_code) {
//                 Ok(data_type) => Ok(DataTypeDeser(data_type)),
//                 Err(_) => Err(D::Error::custom(format!("Invalid datatype: {}", int_code))),
//             },
//             Err(_) => match s.to_lowercase().as_str() {
//                 "boolean" => Ok(DataTypeDeser(DataType::Boolean)),
//                 "int8" => Ok(DataTypeDeser(DataType::Int8)),
//                 "int16" => Ok(DataTypeDeser(DataType::Int16)),
//                 "int32" => Ok(DataTypeDeser(DataType::Int32)),
//                 "uint8" => Ok(DataTypeDeser(DataType::UInt8)),
//                 "uint16" => Ok(DataTypeDeser(DataType::UInt16)),
//                 "uint32" => Ok(DataTypeDeser(DataType::UInt32)),
//                 "real32" => Ok(DataTypeDeser(DataType::Real32)),
//                 "visiblestring" => Ok(DataTypeDeser(DataType::VisibleString)),
//                 "octetstring" => Ok(DataTypeDeser(DataType::OctetString)),
//                 "unicodestring" => Ok(DataTypeDeser(DataType::UnicodeString)),
//                 "timeofday" => Ok(DataTypeDeser(DataType::TimeOfDay)),
//                 "timedifference" => Ok(DataTypeDeser(DataType::TimeDifference)),
//                 "domain" => Ok(DataTypeDeser(DataType::Domain)),

//                 _ => Err(D::Error::custom(format!("Invalid data type: {}", s))),
//             },
//         }
//     }
// }
// impl From<DataType> for DataTypeDeser {
//     fn from(data_type: DataType) -> Self {
//         DataTypeDeser(data_type)
//     }
// }

// impl From<DataTypeDeser> for DataType {
//     fn from(data_type: DataTypeDeser) -> Self {
//         data_type.0
//     }
// }
