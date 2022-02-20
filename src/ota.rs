use core::fmt;

extern crate alloc;
use alloc::borrow::Cow;
use alloc::string::String;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::errors::Errors;
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

pub trait FirmwareInfoLoader: Errors {
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

pub trait OtaSlot: Errors {
    fn get_label(&self) -> Result<Cow<'_, str>, Self::Error>;
    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

pub trait Ota: Errors {
    type Slot<'a>: OtaSlot
    where
        Self: 'a;
    type Update<'a>: OtaUpdate
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

#[derive(Debug)]
pub enum OtaUpdateError<O, R>
where
    O: fmt::Display + fmt::Debug,
    R: fmt::Display + fmt::Debug,
{
    UpdateError(O),
    ReadError(R),
}

impl<O, R> fmt::Display for OtaUpdateError<O, R>
where
    O: fmt::Display + fmt::Debug,
    R: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OtaUpdateError::UpdateError(o) => write!(f, "Update Error {}", o),
            OtaUpdateError::ReadError(r) => write!(f, "Read Error {}", r),
        }
    }
}

#[cfg(feature = "std")]
impl<O, R> std::error::Error for OtaUpdateError<O, R>
where
    O: fmt::Display + fmt::Debug,
    R: fmt::Display + fmt::Debug,
    // TODO
    // where
    //     S: std::error::Error + 'static,
    //     W: std::error::Error + 'static,
{
    // TODO
    // fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    //     match self {
    //         SendError::SendError(s) => Some(s),
    //         SendError::WriteError(w) => Some(w),
    //     }
    // }
}

pub trait OtaUpdate: io::Write {
    fn complete(self) -> Result<(), Self::Error>;
    fn abort(self) -> Result<(), Self::Error>;

    fn update<R>(
        mut self,
        read: R,
        progress: impl Fn(u64, u64),
    ) -> Result<(), OtaUpdateError<Self::Error, R::Error>>
    where
        R: io::Read,
        Self: Sized,
    {
        match io::copy_len_with_progress(read, &mut self, u64::MAX, progress) {
            Ok(_) => self.complete().map_err(OtaUpdateError::UpdateError),
            Err(e) => {
                self.abort().map_err(OtaUpdateError::UpdateError)?;

                let e = match e {
                    io::CopyError::ReadError(e) => OtaUpdateError::ReadError(e),
                    io::CopyError::WriteError(e) => OtaUpdateError::UpdateError(e),
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
    type Iterator: Iterator<Item = FirmwareInfo>;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error>;

    fn get_releases(&mut self) -> Result<Self::Iterator, Self::Error>;

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error>;
}
