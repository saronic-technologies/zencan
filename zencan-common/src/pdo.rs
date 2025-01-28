use core::cell::RefCell;

pub struct Pdo<'a> {
    cob_id_word: u32,
    transmission_type: u8,
    inhibit_time: u16,
    event_timer: u16,
    sync_start: u8,
    buffered_value: [u8; 64],
    mapping_params: RefCell<&'a [u32]>,
}
