extern crate alloc;
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use async_trait::async_trait;

use either::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::io;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct FirmwareInfo {
    pub version: String,
    pub released: String,
    pub description: String,
    pub signature: Option<alloc::vec::Vec<u8>>,
    pub download_id: Option<String>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct UpdateProgress {
    pub progress: f32,
    pub operation: String,
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

pub trait OtaSlot {
    type Error;

    fn get_label(&self) -> Result<Cow<'_, str>, Self::Error>;
    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

pub trait Ota {
    type Slot<'a>: OtaSlot;
    type Update<'a>: OtaUpdate;
    type Error;

    fn get_boot_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn get_running_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn get_update_slot(&self) -> Result<Self::Slot<'_>, Self::Error>;

    fn is_factory_reset_supported(&self) -> Result<bool, Self::Error>;

    fn factory_reset(&mut self) -> Result<(), Self::Error>;

    fn initiate_update(&mut self) -> Result<Self::Update<'_>, Self::Error>;

    fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error>;
    fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
}

pub trait OtaUpdate: io::Write {
    fn complete(self) -> Result<(), Self::Error>;
    fn abort(self) -> Result<(), Self::Error>;

    fn update<R>(
        mut self,
        read: R,
        progress: impl Fn(u64, u64),
    ) -> Result<(), Either<R::Error, Self::Error>>
    where
        R: io::Read,
        Self: Sized,
    {
        match io::copy_len_with_progress(read, &mut self, u64::MAX, progress) {
            Ok(_) => self.complete().map_err(Either::Right),
            Err(e) => {
                self.abort().map_err(Either::Right)?;
                Err(e)
            }
        }
    }
}

#[async_trait]
pub trait OtaAsync {
    type Error;

    async fn get_available_update(&self) -> Result<Option<FirmwareInfo>, Self::Error>;

    async fn get_all_updates(&self) -> Result<Vec<FirmwareInfo>, Self::Error>;

    async fn factory_reset(&mut self) -> Result<(), Self::Error>;

    async fn update(&mut self, download_id: Option<String>) -> Result<(), Self::Error>;

    async fn get_update_progress(&mut self) -> Result<Option<UpdateProgress>, Self::Error>;
}

pub trait OtaRead: io::Read {
    fn size(&self) -> Option<usize>;
}

pub trait OtaServer {
    type OtaRead<'a>: OtaRead<Error = Self::Error>;
    type Iterator: Iterator<Item = FirmwareInfo>;
    type Error;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error>;

    fn get_releases(&mut self) -> Result<Self::Iterator, Self::Error>;

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error>;
}
