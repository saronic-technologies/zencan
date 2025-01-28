//! Exports an example node definition created from an EDS

use zencan_common::objects::ObjectDict;

zencan_node::include_modules!(EXAMPLE2);

pub fn get_od() -> ObjectDict<'static, 'static, 34> {
    ObjectDict::new(&OD_TABLE)
}
