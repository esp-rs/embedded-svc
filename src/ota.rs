#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::errors::{EitherError, Errors};
use crate::io;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct FirmwareInfo<'a> {
    pub version: &'a str,
    pub released: &'a str,
    pub description: &'a str,
    pub signature: Option<&'a [u8]>,
    pub download_id: Option<&'a str>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct UpdateProgress<'a> {
    pub progress: f32,
    pub operation: &'a str,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum LoadResult {
    ReloadMore,
    LoadMore,
    Loaded,
}

pub trait FirmwareInfoLoader: Errors {
    fn load(&mut self, buf: &[u8]) -> Result<LoadResult, Self::Error>;

    fn is_loaded(&self) -> bool;

    fn get_info(&self) -> Result<FirmwareInfo<'_>, Self::Error>;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum SlotState {
    Valid,
    Invalid,
    Unverified,
    Unknown,
}

pub trait OtaSlot: Errors {
    fn get_label(&self) -> Result<&'_ str, Self::Error>;
    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo<'_>>, Self::Error>;
}

pub trait Ota: Errors {
    type Slot<'a>: OtaSlot<Error = Self::Error>
    where
        Self: 'a;

    type Update<'a>: OtaUpdate<Error = Self::Error>
    where
        Self: 'a;

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
    ) -> Result<(), EitherError<Self::Error, R::Error>>
    where
        R: io::Read,
        Self: Sized,
    {
        match io::copy_len_with_progress::<64, _, _, _>(read, &mut self, u64::MAX, progress) {
            Ok(_) => self.complete().map_err(EitherError::First),
            Err(e) => {
                self.abort().map_err(EitherError::First)?;

                let e = match e {
                    EitherError::First(e) => EitherError::Second(e),
                    EitherError::Second(e) => EitherError::First(e),
                };

                Err(e)
            }
        }
    }
}

pub trait OtaRead: io::Read {
    fn size(&self) -> Option<usize>;
}

pub trait OtaServer: Errors {
    type OtaRead<'a>: OtaRead<Error = Self::Error>
    where
        Self: 'a;

    type Iterator<'a>: Iterator<Item = FirmwareInfo<'a>>
    where
        Self: 'a;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo<'_>>, Self::Error>;

    fn get_releases(&mut self) -> Result<Self::Iterator<'_>, Self::Error>;

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error>;
}
