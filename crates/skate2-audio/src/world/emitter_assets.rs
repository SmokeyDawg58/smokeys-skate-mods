//! `Sk8::Audio::AudioAssetManager` as the audio system uses it: the swap banks (asset banks
//! 32-37) whose slots take the emitter banks of `data\audio\emitters.big` and the DLC emitter
//! data, the fixed asset banks `AsyncLoadAt` loads into memory the caller set aside (the traffic
//! banks 10 and 11, the crowd bank 14), the slots' `tAudioAssetInfo`, the async load queue and
//! the unload list (`Update` 0x82a7a900: `UpdateUnloads`, `UpdateAsyncLoads`,
//! `UpdateUnloadsFromSwapBank`).
//!
//! The file loader reads from memory: a load started by `UpdateAsyncLoads` completes on its next
//! call, where natively the disc read takes one or more updates. A bank unloaded without
//! `unresolveData` stays added to AEMS until its slot loads another file, as natively.

use crate::aems::Aems;
use crate::rwac::AudioCore;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// `eAssetStatus`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AssetStatus {
    #[default]
    Empty,
    Loaded,
    Loading,
    Unloading,
}

/// `eAssetType`.
pub const ASSET_DATA: u32 = 0;
pub const ASSET_AEMS_BANK: u32 = 1;
pub const ASSET_SPLICER_DATA: u32 = 2;
const ASSET_MAX: u32 = 3;

/// `mSwapBanks[6]`; a swap bank's slots are asset bank `32 + index`.
const NUM_SWAP_BANKS: usize = 6;
const MAX_SWAP_SLOTS: usize = 5;

/// `tAudioAssetInfo` (100 bytes).
#[derive(Debug, Clone)]
struct AssetInfo {
    status: AssetStatus,
    kind: u32,
    ref_count: i32,
    filename: String,
    data: Option<Arc<[u8]>>,
    handle: i32,
    pre_allocated: bool,
    unload_count: i32,
    resolved: bool,
}

impl Default for AssetInfo {
    fn default() -> Self {
        Self { status: AssetStatus::Empty, kind: ASSET_DATA, ref_count: 0, filename: String::new(), data: None, handle: 0, pre_allocated: false, unload_count: 0, resolved: false }
    }
}

/// `tSwapBankInfo` and its `mSwapFileNames` row.
#[derive(Debug, Clone, Default)]
struct SwapBank {
    used: bool,
    slot_loaded: [bool; MAX_SWAP_SLOTS],
    num_slots: usize,
    file_names: [String; MAX_SWAP_SLOTS],
    slots: [AssetInfo; MAX_SWAP_SLOTS],
}

/// A `tAudioAssetInfo` of `mAssetInfo[bankID][sfxID]`: a swap bank's slot (asset bank
/// `32 + index`) or a slot of a fixed asset bank (below 32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SlotRef {
    Swap(usize, usize),
    Bank(usize, usize),
}

/// `tAsyncInfo`: an `AsyncLoadAtImp` request.
#[derive(Debug, Clone)]
struct AsyncRequest {
    slot: SlotRef,
    kind: u32,
    filename: String,
}

/// The swap-bank part of the `AudioAssetManager` singleton.
#[derive(Default)]
pub struct AudioAssetManager {
    banks: [SwapBank; NUM_SWAP_BANKS],
    /// `mCurrentAsyncList`, head first.
    current: VecDeque<AsyncRequest>,
    /// `mAsyncStatus == eAsyncStatus_Loading` and the read `mpLoadOp` holds (`None` when the file
    /// is missing, which natively fails the load).
    load_op: Option<Option<Arc<[u8]>>>,
    /// `mUnloadList`: the slots waiting for their AEMS bank to leave.
    unload_list: Vec<SlotRef>,
    /// The fixed asset banks' slots by (bank, slot).
    bank_slots: HashMap<(usize, usize), AssetInfo>,
    /// The file loader's files by lower-case path (`data\audio\emitters.big|<bank>`).
    files: HashMap<String, Arc<[u8]>>,
}

impl AudioAssetManager {
    #[must_use]
    pub fn new(files: HashMap<String, Arc<[u8]>>) -> Self {
        Self { files, ..Self::default() }
    }

    /// Adds files the loader can read (lower-case paths).
    pub fn add_files(&mut self, files: impl IntoIterator<Item = (String, Arc<[u8]>)>) {
        self.files.extend(files);
    }

    /// The size of a file the loader can read (`rw::core::Big::LocateEntry`'s entry size).
    #[must_use]
    pub fn file_size(&self, file_name: &str) -> Option<usize> {
        self.files.get(&file_name.to_lowercase()).map(|bytes| bytes.len())
    }

    fn asset(&self, slot: SlotRef) -> Option<&AssetInfo> {
        match slot {
            SlotRef::Swap(bank, slot) => Some(&self.banks[bank].slots[slot]),
            SlotRef::Bank(bank, slot) => self.bank_slots.get(&(bank, slot)),
        }
    }

    fn asset_mut(&mut self, slot: SlotRef) -> &mut AssetInfo {
        match slot {
            SlotRef::Swap(bank, slot) => &mut self.banks[bank].slots[slot],
            SlotRef::Bank(bank, slot) => self.bank_slots.entry((bank, slot)).or_default(),
        }
    }

    /// `AsyncLoadAt` 0x82a7a9b8 (`AsyncLoadAtImp` 0x82a79d98, `AsyncLoadImp` 0x82a79bc8) into a
    /// fixed asset bank's slot: the slot's resolved AEMS bank starts leaving, and the load is
    /// queued into the caller's memory (`preAllocated`).
    pub fn async_load_at(&mut self, file_name: &str, bank: usize, slot: usize, kind: u32, aems: &mut Aems, core: &mut AudioCore) {
        let slot = SlotRef::Bank(bank, slot);
        let asset = self.asset_mut(slot);
        asset.status = AssetStatus::Loading;
        asset.ref_count = 1;
        if asset.resolved && asset.kind == ASSET_AEMS_BANK {
            aems.begin_remove_module_bank(core, asset.handle);
        }
        asset.pre_allocated = true;
        self.current.push_back(AsyncRequest { slot, kind, filename: file_name.to_owned() });
    }

    /// `AsyncStatus` 0x82a792f8 (`AsyncStatusImp` 0x82a78630) of a fixed asset bank's slot.
    #[must_use]
    pub fn async_status(&self, bank: usize, slot: usize) -> AssetStatus {
        self.asset(SlotRef::Bank(bank, slot)).map_or(AssetStatus::Empty, |asset| asset.status)
    }

    /// `UnLoad` 0x82a7aa38: `UnLoadImp` of a fixed asset bank's slot with `unresolveData`.
    pub fn unload_bank_slot(&mut self, bank: usize, slot: usize, aems: &mut Aems, core: &mut AudioCore) {
        self.unload(SlotRef::Bank(bank, slot), true, aems, core);
    }

    /// `CreateSwapBank` 0x82a78860 / 0x82a78988: the first empty bank; every bank's file names
    /// are cleared, as natively.
    pub fn create_swap_bank(&mut self, num_slots: usize) -> usize {
        let index = self.banks.iter().position(|bank| !bank.used).unwrap_or(NUM_SWAP_BANKS - 1);
        let bank = &mut self.banks[index];
        bank.used = true;
        bank.num_slots = num_slots.min(MAX_SWAP_SLOTS);
        bank.slot_loaded = [false; MAX_SWAP_SLOTS];
        for bank in &mut self.banks {
            for name in &mut bank.file_names {
                name.clear();
            }
        }
        index
    }

    /// `FindFreeSlot` 0x82a79408: an unloaded slot whose last file is `file_name`, else the
    /// first unloaded slot in `direction`; -1 when none is free or a queued load targets it.
    pub fn find_free_slot(&self, bank: usize, backwards: bool, file_name: Option<&str>) -> i32 {
        let swap = &self.banks[bank];
        let count = swap.num_slots;
        let mut slot = count;
        if let Some(name) = file_name {
            slot = (0..count).find(|&i| !swap.slot_loaded[i] && swap.file_names[i] == name).unwrap_or(count);
        }
        if slot == count {
            let found = if backwards { (0..count).rev().find(|&i| !swap.slot_loaded[i]) } else { (0..count).find(|&i| !swap.slot_loaded[i]) };
            match found {
                Some(i) => slot = i,
                None => return -1,
            }
        }
        if self.current.iter().any(|request| request.slot == SlotRef::Swap(bank, slot)) {
            return -1;
        }
        slot as i32
    }

    /// `AsyncLoadToSwapSlot` 0x82a7a340: a slot still resolved with the same file is marked
    /// loaded again (`ResolvePreloadedData` 0x82a78578); otherwise the load is queued
    /// (`AsyncLoadImp` 0x82a79bc8 starts removing the slot's previous AEMS bank).
    pub fn async_load_to_swap_slot(&mut self, file_name: &str, bank: usize, slot: usize, kind: u32, aems: &mut Aems, core: &mut AudioCore) {
        let swap = &mut self.banks[bank];
        swap.slot_loaded[slot] = true;
        let asset = &mut swap.slots[slot];
        if asset.resolved && swap.file_names[slot] == file_name {
            asset.pre_allocated = true;
            asset.kind = kind;
            asset.status = AssetStatus::Loaded;
            asset.ref_count = 1;
            return;
        }
        // `AsyncLoadAtImp` 0x82a79d98 loads into the slot's memory (`preAllocated`).
        asset.pre_allocated = true;
        asset.status = AssetStatus::Loading;
        asset.ref_count = 1;
        if asset.resolved && asset.kind == ASSET_AEMS_BANK {
            aems.begin_remove_module_bank(core, asset.handle);
        }
        self.current.push_back(AsyncRequest { slot: SlotRef::Swap(bank, slot), kind, filename: file_name.to_owned() });
        self.banks[bank].file_names[slot] = file_name.to_owned();
    }

    /// `AsyncToSwapSlotStatus` 0x82a78a40.
    #[must_use]
    pub fn async_to_swap_slot_status(&self, bank: usize, slot: usize) -> AssetStatus {
        self.banks[bank].slots[slot].status
    }

    /// `UnloadFromSwapSlot` 0x82a7a598 (`UnLoadImp` 0x82a7a1d0): without `unresolve` an AEMS
    /// bank empties the slot at once and stays added; with it the bank starts leaving AEMS and
    /// the slot empties in `UpdateUnloads`.
    pub fn unload_from_swap_slot(&mut self, bank: usize, slot: usize, unresolve: bool, aems: &mut Aems, core: &mut AudioCore) {
        if !self.banks[bank].slot_loaded[slot] {
            return;
        }
        self.unload(SlotRef::Swap(bank, slot), unresolve, aems, core);
        if self.banks[bank].slots[slot].status == AssetStatus::Empty {
            self.banks[bank].slot_loaded[slot] = false;
        }
    }

    /// `UnLoadImp` 0x82a7a1d0.
    fn unload(&mut self, slot: SlotRef, unresolve: bool, aems: &mut Aems, core: &mut AudioCore) {
        let asset = self.asset_mut(slot);
        if asset.ref_count == 0 {
            return;
        }
        asset.ref_count -= 1;
        if asset.ref_count > 0 {
            return;
        }
        if asset.status == AssetStatus::Loading {
            asset.status = AssetStatus::Unloading;
            return;
        }
        if unresolve && (asset.kind == ASSET_AEMS_BANK || asset.kind == ASSET_SPLICER_DATA) {
            asset.status = AssetStatus::Unloading;
            if asset.kind == ASSET_AEMS_BANK {
                aems.begin_remove_module_bank(core, asset.handle);
            }
            self.unload_list.push(slot);
            return;
        }
        asset.status = AssetStatus::Empty;
        asset.ref_count = 0;
        asset.data = None;
        asset.pre_allocated = false;
        asset.unload_count = 3;
        asset.filename.clear();
    }

    /// `AllSwapSlotsUnloaded` 0x82a78c10.
    #[must_use]
    pub fn all_swap_slots_unloaded(&self, bank: usize) -> bool {
        let swap = &self.banks[bank];
        swap.slot_loaded[..swap.num_slots].iter().all(|&loaded| !loaded)
    }

    /// `UnresolveAllSwapSlots` 0x82a78ea8: every resolved slot's bank starts leaving AEMS.
    pub fn unresolve_all_swap_slots(&mut self, bank: usize, aems: &mut Aems, core: &mut AudioCore) {
        let swap = &self.banks[bank];
        for asset in &swap.slots[..swap.num_slots] {
            if asset.resolved {
                aems.begin_remove_module_bank(core, asset.handle);
            }
        }
    }

    /// `AreAllSwapSlotsUnresolved` 0x82a78f98: slots whose bank has left are cleared.
    pub fn are_all_swap_slots_unresolved(&mut self, bank: usize, aems: &Aems) -> bool {
        let mut result = true;
        let swap = &mut self.banks[bank];
        let count = swap.num_slots;
        for asset in &mut swap.slots[..count] {
            if !asset.resolved {
                continue;
            }
            if aems.is_module_bank_removed(asset.handle) {
                asset.kind = ASSET_MAX;
                asset.resolved = false;
                asset.handle = 0;
            } else {
                result = false;
            }
        }
        result
    }

    /// `GetSwapData` 0x82a78db8.
    #[must_use]
    pub fn swap_data(&self, bank: usize, slot: usize) -> Option<Arc<[u8]>> {
        self.banks[bank].slots[slot].data.clone()
    }

    /// `UnloadAllSwapSlots` 0x82a7a6b8.
    pub fn unload_all_swap_slots(&mut self, bank: usize, aems: &mut Aems, core: &mut AudioCore) {
        for slot in 0..self.banks[bank].num_slots {
            if self.banks[bank].slot_loaded[slot] {
                self.unload(SlotRef::Swap(bank, slot), true, aems, core);
                if self.banks[bank].slots[slot].status == AssetStatus::Empty {
                    self.banks[bank].slot_loaded[slot] = false;
                }
            }
        }
    }

    /// `Update` 0x82a7a900 (`UpdateFreeSwapBank` has nothing to free: no swap bank is freed).
    pub fn update(&mut self, aems: &mut Aems, core: &mut AudioCore) {
        self.update_unloads(aems);
        self.update_async_loads(aems, core);
        for swap in &mut self.banks {
            for slot in 0..swap.num_slots {
                if swap.slot_loaded[slot] && swap.slots[slot].status == AssetStatus::Empty {
                    swap.slot_loaded[slot] = false;
                }
            }
        }
    }

    /// `UpdateUnloads` 0x82a796e0: AEMS banks that have left AEMS empty their slots; splicer
    /// data waits three updates.
    fn update_unloads(&mut self, aems: &Aems) {
        let mut index = 0;
        while index < self.unload_list.len() {
            let slot = self.unload_list[index];
            let asset = self.asset_mut(slot);
            let done = match asset.kind {
                ASSET_AEMS_BANK => aems.is_module_bank_removed(asset.handle),
                ASSET_SPLICER_DATA => {
                    asset.unload_count -= 1;
                    asset.unload_count <= 0
                }
                _ => {
                    index += 1;
                    continue;
                }
            };
            if done {
                *asset = AssetInfo { kind: ASSET_MAX, unload_count: 3, ..AssetInfo::default() };
                self.unload_list.remove(index);
            } else {
                index += 1;
            }
        }
    }

    /// `UpdateAsyncLoads` 0x82a79e70: the queue's head loads once its slot's old bank has left
    /// AEMS; a finished load resolves (`ResolveData` 0x82a786b0 adds an AEMS bank).
    fn update_async_loads(&mut self, aems: &mut Aems, core: &mut AudioCore) {
        if let Some(read) = self.load_op.take() {
            let Some(request) = self.current.pop_front() else { return };
            let asset = self.asset_mut(request.slot);
            if asset.status == AssetStatus::Loading {
                asset.data = read;
                asset.status = AssetStatus::Loaded;
                asset.kind = request.kind;
                asset.filename = request.filename.chars().take(63).collect();
                if asset.kind == ASSET_AEMS_BANK {
                    if let Some(data) = asset.data.clone() {
                        if let Some(handle) = aems.add_module_bank(core, &request.filename, data) {
                            asset.handle = handle;
                            asset.resolved = true;
                        }
                    }
                }
            } else if asset.status == AssetStatus::Unloading {
                *asset = AssetInfo { kind: asset.kind, handle: asset.handle, resolved: asset.resolved, unload_count: 3, ..AssetInfo::default() };
            }
            return;
        }
        let Some(request) = self.current.front().cloned() else { return };
        let asset = self.asset_mut(request.slot);
        if asset.resolved {
            if asset.kind == ASSET_AEMS_BANK {
                if !aems.is_module_bank_removed(asset.handle) {
                    return;
                }
            } else {
                asset.unload_count -= 1;
                if asset.unload_count > 0 {
                    return;
                }
            }
            asset.handle = 0;
            asset.kind = ASSET_MAX;
            asset.resolved = false;
        }
        let read = self.files.get(&request.filename.to_lowercase()).cloned();
        self.load_op = Some(read);
    }
}
