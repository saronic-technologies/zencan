//! Implements node state struct
use zencan_common::messages::CanId;
use zencan_common::objects::{find_object, ODEntry, ObjectFlagSync, ObjectRawAccess as _};
use zencan_common::AtomicCell;

use crate::storage::StorageContext;

/// Specifies the number of mapping parameters supported per PDO
///
/// Since we do not yet support CAN-FD, or sub-byte mapping, it's not possible to map more than 8
/// objects to a single PDO
const N_MAPPING_PARAMS: usize = 8;

/// Represents a single PDO state
#[derive(Debug)]
pub struct Pdo {
    /// The COB-ID used to send or receive this PDO
    pub cob_id: AtomicCell<CanId>,
    /// Indicates if the PDO is enabled
    pub valid: AtomicCell<bool>,
    /// If set, this PDO cannot be requested via RTR
    pub rtr_disabled: AtomicCell<bool>,
    /// Transmission type field (subindex 0x2)
    /// Determines when the PDO is sent/received
    ///
    /// 0 (unused): PDO is sent on receipt of SYNC, but only if the event has been triggered
    /// 1 - 240: PDO is sent on receipt of every Nth SYNC message
    /// 254: PDO is sent asynchronously on application request
    pub transmission_type: AtomicCell<u8>,
    /// Tracks the number of sync signals since this was last sent or received
    pub sync_counter: AtomicCell<u8>,
    /// Inhibit time for this PDO in us
    pub inhibit_time: AtomicCell<u16>,
    /// The last received data value for an RPDO
    pub buffered_value: AtomicCell<Option<[u8; 8]>>,
    /// The mapping parameters
    ///
    /// These specify which objects are
    pub mapping_params: [AtomicCell<u32>; N_MAPPING_PARAMS],
}

impl Default for Pdo {
    fn default() -> Self {
        Self::new()
    }
}

impl Pdo {
    /// Create a new PDO object
    pub const fn new() -> Self {
        let cob_id = AtomicCell::new(CanId::Std(0));
        let valid = AtomicCell::new(false);
        let rtr_disabled = AtomicCell::new(false);
        let transmission_type = AtomicCell::new(0);
        let sync_counter = AtomicCell::new(0);
        let inhibit_time = AtomicCell::new(0);
        let buffered_value = AtomicCell::new(None);
        let mapping_params = [const { AtomicCell::new(0) }; N_MAPPING_PARAMS];
        Self {
            cob_id,
            valid,
            rtr_disabled,
            transmission_type,
            sync_counter,
            inhibit_time,
            buffered_value,
            mapping_params,
        }
    }

    /// This function should be called when a SYNC event occurs
    ///
    /// It will return true if the PDO should be sent in response to the SYNC event
    pub fn sync_update(&self) -> bool {
        if !self.valid.load() {
            return false;
        }

        let transmission_type = self.transmission_type.load();
        if transmission_type == 0 {
            // TODO: Figure out how to determine application "event" which triggers the PDO
            // For now, send every sync
            true
        } else if transmission_type <= 240 {
            let cnt = self.sync_counter.fetch_add(1) + 1;
            cnt == transmission_type
        } else {
            false
        }
    }

    /// Check mapped objects for TPDO event flag
    pub fn read_events(&self, od: &[ODEntry]) -> bool {
        // TODO: Should maybe cache pointers or something. This is searching the whole OD for every
        // mapped object
        if !self.valid.load() {
            return false;
        }

        for i in 0..self.mapping_params.len() {
            let param = self.mapping_params[i].load();
            if param == 0 {
                break;
            }
            let object_id = (param >> 16) as u16;
            let sub_index = ((param & 0xFF00) >> 8) as u8;
            // Unwrap safety: Object is validated to exist prior to setting mapping
            let entry = find_object(od, object_id).expect("invalid mapping parameter");
            if entry.read_event_flag(sub_index) {
                return true;
            }
        }
        false
    }

    pub(crate) fn clear_events(&self, od: &[ODEntry]) {
        for i in 0..self.mapping_params.len() {
            let param = self.mapping_params[i].load();
            if param == 0 {
                break;
            }
            let object_id = (param >> 16) as u16;
            // Unwrap safety: Object is validated to exist prior to setting mapping
            let entry = find_object(od, object_id).expect("invalid mapping parameter");
            entry.clear_events();
        }
    }
}

/// A trait by which NodeState is accessed
///
/// TODO: This should probably be sealed
pub trait NodeStateAccess: Sync + Send {
    /// Get the receive PDO objects
    fn get_rpdos(&self) -> &[Pdo];
    /// Get the transmit PDO objects
    fn get_tpdos(&self) -> &[Pdo];
    /// Get the PDO flag sync object
    fn get_pdo_sync(&self) -> &ObjectFlagSync;
    /// Get the storage context object
    fn storage_context(&self) -> &StorageContext;
}

/// The NodeState provides config-dependent storage to the [`Node`](crate::Node) object
///
/// The node state has to get instantiated (statically) by zencan-build, based on the device config
/// file. It is then provided to the node by the application when it is instantiated, and accessed
/// via the [`NodeStateAccess`] trait.
pub struct NodeState<const N_RPDO: usize, const N_TPDO: usize> {
    rpdos: [Pdo; N_RPDO],
    tpdos: [Pdo; N_TPDO],
    pdo_sync: ObjectFlagSync,
    storage_context: StorageContext,
}

impl<const N_RPDO: usize, const N_TPDO: usize> Default for NodeState<N_RPDO, N_TPDO> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N_RPDO: usize, const N_TPDO: usize> NodeState<N_RPDO, N_TPDO> {
    /// Create a new NodeState object
    pub const fn new() -> Self {
        let rpdos = [const { Pdo::new() }; N_RPDO];
        let tpdos = [const { Pdo::new() }; N_TPDO];
        let pdo_sync = ObjectFlagSync::new();
        let storage_context = StorageContext::new();
        Self {
            rpdos,
            tpdos,
            pdo_sync,
            storage_context,
        }
    }

    /// Access the RPDOs as a const function
    ///
    /// This is required so that they can be shared with the NodeMbox object in generated code
    pub const fn rpdos(&'static self) -> &'static [Pdo] {
        &self.rpdos
    }

    /// Access the pdo_sync as a const function
    ///
    /// This is required so that it can be shared with the objects in generated code
    pub const fn pdo_sync(&'static self) -> &'static ObjectFlagSync {
        &self.pdo_sync
    }
}

impl<const N_RPDO: usize, const N_TPDO: usize> NodeStateAccess for NodeState<N_RPDO, N_TPDO> {
    fn get_rpdos(&self) -> &[Pdo] {
        &self.rpdos
    }

    fn get_tpdos(&self) -> &[Pdo] {
        &self.tpdos
    }

    fn get_pdo_sync(&self) -> &ObjectFlagSync {
        &self.pdo_sync
    }

    fn storage_context(&self) -> &StorageContext {
        &self.storage_context
    }
}
