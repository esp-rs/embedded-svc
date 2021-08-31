use async_trait::async_trait;

#[cfg(feature = "alloc")]
extern crate alloc;

#[derive(Clone, Debug)]
pub struct FirmwareInfo {
    pub version: String,
    pub released: String,
    pub description: String,
    #[cfg(feature = "alloc")]
    pub signature: Option<alloc::vec::Vec<u8>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum LoadResult {
    ReloadMore,
    LoadMore,
    Loaded,
}

pub trait FirmwareInfoLoader {
    type Error;

    fn load(&mut self, buf: &[u8]) -> Result<LoadResult, Self::Error>;

    fn is_loaded(&self) -> bool;

    fn get_info(&self) -> Result<FirmwareInfo, Self::Error>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum SlotState {
    Valid,
    Invalid,
    Unverified,
    Unknown,
}

pub trait Slot {
    type Error;

    fn get_label(&self) -> Result<String, Self::Error>;
    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

pub trait Ota {
    type Slot: Slot;
    type OtaUpdate: OtaUpdate;
    type Error;

    fn get_boot_slot<'a>(&'a self) -> Result<Self::Slot, Self::Error>
    where
        Self::Slot: 'a;

    fn get_running_slot<'a>(&'a self) -> Result<Self::Slot, Self::Error>
    where
        Self::Slot: 'a;

    fn get_update_slot<'a>(&'a self) -> Result<Self::Slot, Self::Error>
    where
        Self::Slot: 'a;

    fn factory_reset(self) -> Self::Error; // TODO: Figure out how to report on status update

    fn initiate_update(self) -> Result<Self::OtaUpdate, Self::Error>;

    fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error>;
    fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
}

#[cfg(feature = "std")]
pub trait OtaUpdate: std::io::Write {
    type Ota: Ota;
    type Error;

    fn write_buf(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

    fn complete(self) -> Result<Self::Ota, Self::Error>;
    fn abort(self) -> Result<Self::Ota, Self::Error>;
}

#[cfg(not(feature = "std"))]
pub trait OtaUpdate {
    type Ota: Ota;
    type Error;

    fn write_buf(&mut self, buf: &[u8]) -> Result<(), Self::Error>;

    fn complete(self) -> Result<Self::Ota, Self::Error>;
    fn abort(self) -> Result<Self::Ota, Self::Error>;
}

#[async_trait]
pub trait OtaService {
    type Error;

    async fn get_available_update(&self) -> Result<Option<String>, Self::Error>;

    async fn factory_reset(&mut self) -> Self::Error;

    async fn update(&mut self) -> Self::Error;
}
