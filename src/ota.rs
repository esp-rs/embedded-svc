extern crate alloc;
use alloc::borrow::Cow;

use async_trait::async_trait;

use crate::io;

#[derive(Clone, Debug)]
pub struct FirmwareInfo {
    pub version: String,
    pub released: String,
    pub description: String,
    #[cfg(feature = "alloc")]
    pub signature: Option<alloc::vec::Vec<u8>>,
    pub download_id: Option<String>,
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

pub trait Slot<'a> {
    type Error;

    fn get_label(&self) -> Result<Cow<'_, str>, Self::Error>;
    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

pub trait Ota {
    type Slot<'a>: Slot<'a>;
    type OtaUpdate: OtaUpdate;
    type Error;

    fn get_boot_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn get_running_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn get_update_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn factory_reset(self) -> Self::Error; // TODO: Figure out how to report on status update

    fn initiate_update(self) -> Result<Self::OtaUpdate, Self::Error>;

    fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error>;
    fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
}

pub trait OtaUpdate: io::Write {
    type Ota: Ota;

    fn complete(self) -> Result<Self::Ota, Self::Error>;
    fn abort(self) -> Result<Self::Ota, Self::Error>;
}

#[async_trait]
pub trait OtaAsync {
    type Error;

    async fn get_available_update(&self) -> Result<Option<String>, Self::Error>;

    async fn factory_reset(&mut self) -> Self::Error;

    async fn update(&mut self) -> Self::Error;
}

pub trait OtaServer {
    type Read<'a>: io::Read<Error = Self::Error>;
    type Iterator: Iterator<Item = FirmwareInfo>;
    type Error;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error>;

    fn get_releases(&mut self) -> Result<Self::Iterator, Self::Error>;

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::Read<'_>, Self::Error>;
}
