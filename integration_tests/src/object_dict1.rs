//! Exports an example node definition created from an EDS

use canopen_common::objects::ObjectDict;

canopen_node::include_modules!(EXAMPLE1);

pub fn get_od() -> ObjectDict<'static> {
    ObjectDict { table: &OD_TABLE }
}
