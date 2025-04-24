use crossbeam::atomic::AtomicCell;
use zencan_common::traits::CanId;

#[derive(Debug)]
pub struct Pdo {
    pub cob_id: AtomicCell<CanId>,
    pub valid: AtomicCell<bool>,
    pub rtr_disabled: AtomicCell<bool>,
    pub transmission_type: AtomicCell<u8>,
    pub inhibit_time: AtomicCell<u16>,
    pub event_timer: AtomicCell<u16>,
    pub sync_start: AtomicCell<u8>,
    pub buffered_value: AtomicCell<Option<[u8; 8]>>,
    pub mapping_params: [AtomicCell<u32>; 32],
}

impl Pdo {
    pub const fn new() -> Self {
        let cob_id = AtomicCell::new(CanId::Std(0));
        let valid = AtomicCell::new(false);
        let rtr_disabled = AtomicCell::new(false);
        let transmission_type = AtomicCell::new(0);
        let inhibit_time = AtomicCell::new(0);
        let event_timer = AtomicCell::new(0);
        let sync_start = AtomicCell::new(0);
        let buffered_value = AtomicCell::new(None);
        let mapping_params = [const { AtomicCell::new(0) }; 32];
        Self {
            cob_id,
            valid,
            rtr_disabled,
            transmission_type,
            inhibit_time,
            event_timer,
            sync_start,
            buffered_value,
            mapping_params,
        }
    }
}

pub trait NodeStateAccess {
    fn num_rpdos(&self) -> usize;
    fn get_rpdos(&self) -> &[Pdo];
    fn num_tpdos(&self) -> usize;
    fn get_tpdos(&self) -> &[Pdo];
}

pub struct NodeState<const N_RPDO: usize, const N_TPDO: usize> {
    pub rpdos: [Pdo; N_RPDO],
    pub tpdos: [Pdo; N_TPDO],
}

impl<const N_RPDO: usize, const N_TPDO: usize> NodeState<N_RPDO, N_TPDO> {
    pub const fn new() -> Self {
        let rpdos = [const { Pdo::new() }; N_RPDO];
        let tpdos = [const { Pdo::new() }; N_TPDO];
        Self { rpdos, tpdos }
    }

    pub const fn rpdos(&'static self) -> &'static [Pdo] {
        &self.rpdos
    }
}

impl<const N_RPDO: usize, const N_TPDO: usize> NodeStateAccess for NodeState<N_RPDO, N_TPDO> {
    fn num_rpdos(&self) -> usize {
        self.rpdos.len()
    }

    fn get_rpdos(&self) -> &[Pdo] {
        &self.rpdos
    }

    fn num_tpdos(&self) -> usize {
        self.tpdos.len()
    }

    fn get_tpdos(&self) -> &[Pdo] {
        &self.tpdos
    }
}