use serde::{de::Error, Deserialize};
use zencan_common::objects::{AccessType, ObjectCode};

use crate::errors::*;
use snafu::ResultExt as _;

pub fn mandatory_objects() -> Vec<ObjectDefinition> {
    vec![
        ObjectDefinition {
            index: 0x1000,
            parameter_name: "Device Type".to_string(),
            application_callback: false,
            object: Object::Var(VarDefinition {
                data_type: DataType::UInt32,
                access_type: AccessType::Const.into(),
                default_value: Some(DefaultValue::Integer(0x00000000)),
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
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: None,
                    },
                    SubDefinition {
                        sub_index: 2,
                        parameter_name: "Product Code".to_string(),
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: None,
                    },
                    SubDefinition {
                        sub_index: 3,
                        parameter_name: "Revision Number".to_string(),
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: None,
                    },
                    SubDefinition {
                        sub_index: 4,
                        parameter_name: "Serial Number".to_string(),
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Const.into(),
                        default_value: None,
                    },
                ],
            }),
        },
    ]
}

pub fn pdo_objects(num_rpdo: usize, num_tpdo: usize) -> Vec<ObjectDefinition> {

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
                    },
                    SubDefinition {
                        sub_index: 2,
                        parameter_name: format!("Transmission type for {}{}", pdo_type, i),
                        field_name: None,
                        data_type: DataType::UInt8,
                        access_type: AccessType::Rw.into(),
                        default_value: None,
                    },
                ],
            }),
        });

        objects.push(ObjectDefinition {
            index: mapping_index + i as u16,
            parameter_name: format!("{}{} Mapping Parameters", pdo_type, i),
            application_callback: true,
            object: Object::Record(RecordDefinition {
                subs: (0..64).map(|j| {
                    SubDefinition {
                        sub_index: j + 1,
                        parameter_name: format!("{}{} Mapping App Object {}", pdo_type, i, j),
                        field_name: None,
                        data_type: DataType::UInt32,
                        access_type: AccessType::Rw.into(),
                        default_value: None,
                    }
                }).collect()
            }),
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

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
/// Device configuration structure
pub struct DeviceConfig {
    #[serde(default)]
    pub vendor_name: String,
    #[serde(default)]
    pub vendor_number: u16,
    /// The number of RX PDO slots available in the device
    pub num_rpdo: u8,
    /// The number of TX PDO slots available in the device
    pub num_tpdo: u8,

    #[serde(default)]
    pub objects: Vec<ObjectDefinition>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct SubDefinition {
    pub sub_index: u8,
    #[serde(default)]
    pub parameter_name: String,
    /// Used to name the struct field associated with this sub object
    ///
    /// This is only applicable to record objects. If no name is provided, the default field name
    /// will be `sub[index]`, where index is the uppercase hex representation of the sub index
    #[serde(default)]
    pub field_name: Option<String>,
    pub data_type: DataType,
    #[serde(default)]
    pub access_type: AccessTypeDeser,
    #[serde(default)]
    pub default_value: Option<DefaultValue>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum DefaultValue {
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "object_type", rename_all = "lowercase")]
pub enum Object {
    Var(VarDefinition),
    Array(ArrayDefinition),
    Record(RecordDefinition),
    Domain(DomainDefinition),
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VarDefinition {
    pub data_type: DataType,
    pub access_type: AccessTypeDeser,
    pub default_value: Option<DefaultValue>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ArrayDefinition {
    pub data_type: DataType,
    pub access_type: AccessTypeDeser,
    pub array_size: usize,
    pub default_value: Option<Vec<DefaultValue>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct RecordDefinition {
    pub subs: Vec<SubDefinition>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DomainDefinition {

}

#[derive(Deserialize, Debug, Clone)]
pub struct ObjectDefinition {
    pub index: u16,
    #[serde(default)]
    pub parameter_name: String,
    #[serde(default)]
    /// If true, this object is implemented by an application callback, and no storage will be
    /// allocated for it in the object dictionary.
    pub application_callback: bool,
    #[serde(flatten)]
    pub object: Object,
}

impl ObjectDefinition {
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
    pub fn load(
        config_path: impl AsRef<std::path::Path>,
    ) -> Result<Self, CompileError> {
        let config_str = std::fs::read_to_string(&config_path).context(IoSnafu)?;
        let mut config: DeviceConfig = toml::from_str(&config_str).context(ParseTomlSnafu {
            message: format!("Error parsing {}", config_path.as_ref().display())})?;

        // Add mandatory objects to the config
        config.objects.extend(mandatory_objects());

        config.objects.extend(pdo_objects(
            config.num_rpdo as usize,
            config.num_tpdo as usize,
        ));

        Ok(config)
    }
}

/// A newtype for ObjectCode to implement deserialization so we can use it in toml files
#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Default, Clone)]
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

#[derive(Debug, Clone, Copy)]
pub enum DataType {
    Boolean,
    Int8,
    Int16,
    Int32,
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
    pub fn is_str(&self) -> bool {
        match self {
            DataType::VisibleString(_) | DataType::OctetString(_) | DataType::UnicodeString(_) => true,
            _ => false,
        }
    }

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
                D::Error::custom(format!(
                    "Invalid size for VisibleString: {}",
                    &caps[1]
                ))
            })?;
            return Ok(DataType::VisibleString(size));
        } else if let Some(caps) = re_octetstring.captures(&s) {
            let size: usize = caps[1].parse().map_err(|_| {
                D::Error::custom(format!(
                    "Invalid size for OctetString: {}",
                    &caps[1]
                ))
            })?;
            return Ok(DataType::OctetString(size));
        } else if let Some(caps) = re_unicodestring.captures(&s) {
            let size: usize = caps[1].parse().map_err(|_| {
                D::Error::custom(format!(
                    "Invalid size for UnicodeString: {}",
                    &caps[1]
                ))
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
