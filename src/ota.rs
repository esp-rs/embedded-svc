use core::convert::TryFrom;
use core::mem::MaybeUninit;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::errors::{conv::StrConvError, wrap::EitherError, Errors};
use crate::io;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct FirmwareInfo<S> {
    pub version: S,
    pub released: S,
    pub description: S,
    pub signature: Option<S>,
    pub download_id: Option<S>,
}

impl<I> FirmwareInfo<I>
where
    I: AsRef<str>,
{
    pub fn try_convert_strings<'a, S>(&'a self) -> Result<FirmwareInfo<S>, StrConvError>
    where
        S: TryFrom<&'a str>,
    {
        Ok(FirmwareInfo {
            version: S::try_from(self.version.as_ref()).map_err(|_| StrConvError)?,
            released: S::try_from(self.released.as_ref()).map_err(|_| StrConvError)?,
            description: S::try_from(self.description.as_ref()).map_err(|_| StrConvError)?,
            signature: if let Some(signature) = &self.signature {
                Some(S::try_from(signature.as_ref()).map_err(|_| StrConvError)?)
            } else {
                None
            },
            download_id: if let Some(download_id) = &self.download_id {
                Some(S::try_from(download_id.as_ref()).map_err(|_| StrConvError)?)
            } else {
                None
            },
        })
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct UpdateProgress<S> {
    pub progress: f32,
    pub operation: S,
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

    fn get_info<S>(&self) -> Result<FirmwareInfo<S>, Self::Error>
    where
        S: for<'a> TryFrom<&'a str>;
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

    fn get_firmware_info<'a, S>(
        &'a self,
    ) -> Result<Option<FirmwareInfo<S>>, EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'a str>;
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
            Ok(_) => self.complete().map_err(EitherError::E1),
            Err(e) => {
                self.abort().map_err(EitherError::E1)?;

                let e = match e {
                    EitherError::E1(e) => EitherError::E2(e),
                    EitherError::E2(e) => EitherError::E1(e),
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

    fn get_latest_release<'a, S>(
        &'a mut self,
    ) -> Result<Option<FirmwareInfo<S>>, EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'a str>;

    fn fill_releases<'a, 'b, S>(
        &'a mut self,
        infos: &'b mut [MaybeUninit<FirmwareInfo<S>>],
    ) -> Result<(&'b [FirmwareInfo<S>], usize), EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'a str>;

    #[cfg(feature = "alloc")]
    fn get_releases(
        &mut self,
    ) -> Result<alloc::vec::Vec<FirmwareInfo<alloc::string::String>>, Self::Error>;

    #[cfg(feature = "heapless")]
    fn get_releases_heapless<'a, S, const N: usize>(
        &'a mut self,
    ) -> Result<heapless::Vec<FirmwareInfo<S>, N>, EitherError<Self::Error, StrConvError>>
    where
        S: TryFrom<&'a str>;

    fn open(&mut self, download_id: impl AsRef<str>) -> Result<Self::OtaRead<'_>, Self::Error>;
}
