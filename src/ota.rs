#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::errors::wrap::EitherError;
use crate::io::{self, Io, Read, Write};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct FirmwareInfo {
    pub version: heapless::String<24>,
    pub released: heapless::String<24>,
    pub description: Option<heapless::String<128>>,
    pub signature: Option<heapless::Vec<u8, 32>>,
    pub download_id: Option<heapless::String<128>>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct UpdateProgress {
    pub progress: u32,
    pub operation: &'static str,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum LoadResult {
    ReloadMore,
    LoadMore,
    Loaded,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
pub enum SlotState {
    Valid,
    Invalid,
    Unverified,
    Unknown,
}

pub trait FirmwareInfoLoader: Io {
    fn load(&mut self, buf: &[u8]) -> Result<LoadResult, Self::Error>;

    fn is_loaded(&self) -> bool;

    fn get_info(&self) -> Result<FirmwareInfo, Self::Error>;
}

pub trait OtaSlot: Io {
    fn get_label(&self) -> Result<&str, Self::Error>;

    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

pub trait Ota: Io {
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

pub trait OtaUpdate: Write {
    fn complete(self) -> Result<(), Self::Error>;

    fn abort(self) -> Result<(), Self::Error>;

    fn update<R>(
        mut self,
        read: R,
        progress: impl Fn(u64, u64),
    ) -> Result<(), EitherError<Self::Error, R::Error>>
    where
        R: Read,
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

pub trait OtaServer: Io {
    type OtaRead<'a>: OtaRead<Error = Self::Error>
    where
        Self: 'a;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error>;

    #[cfg(feature = "alloc")]
    fn get_releases(&mut self) -> Result<alloc::vec::Vec<FirmwareInfo>, Self::Error>;

    fn get_releases_n<const N: usize>(
        &mut self,
    ) -> Result<heapless::Vec<FirmwareInfo, N>, Self::Error>;

    fn open(&mut self, download_id: &str) -> Result<Self::OtaRead<'_>, Self::Error>;
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::errors::wrap::EitherError;
    use crate::io::{self, asynch::Read, asynch::Write, Io};

    pub use super::{FirmwareInfo, FirmwareInfoLoader, LoadResult, SlotState};

    pub trait OtaSlot: Io {
        fn get_label(&self) -> Result<&str, Self::Error>;

        fn get_state(&self) -> Result<SlotState, Self::Error>;

        fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
    }

    pub trait Ota: Io {
        type Slot<'a>: OtaSlot<Error = Self::Error>
        where
            Self: 'a;

        type Update<'a>: OtaUpdate<Error = Self::Error>
        where
            Self: 'a;

        type GetBootSlotFuture<'a>: Future<Output = Result<Self::Slot<'a>, Self::Error>>
        where
            Self: 'a;

        type GetRunningSlotFuture<'a>: Future<Output = Result<Self::Slot<'a>, Self::Error>>
        where
            Self: 'a;

        type GetUpdateSlotFuture<'a>: Future<Output = Result<Self::Slot<'a>, Self::Error>>
        where
            Self: 'a;

        type FactoryResetFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type InitiateUpdateFuture<'a>: Future<Output = Result<Self::Update<'a>, Self::Error>>
        where
            Self: 'a;

        type MarkRunningSlotValidFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        fn get_boot_slot(&self) -> Self::GetBootSlotFuture<'_>;

        fn get_running_slot(&self) -> Self::GetRunningSlotFuture<'_>;

        fn get_update_slot(&self) -> Self::GetUpdateSlotFuture<'_>;

        fn is_factory_reset_supported(&self) -> Result<bool, Self::Error>;

        fn factory_reset(&mut self) -> Self::FactoryResetFuture<'_>;

        fn initiate_update(&mut self) -> Self::InitiateUpdateFuture<'_>;

        fn mark_running_slot_valid(&mut self) -> Self::MarkRunningSlotValidFuture<'_>;

        fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
    }

    pub trait OtaUpdate: Write {
        type CompleteFuture: Future<Output = Result<(), Self::Error>>;

        type AbortFuture: Future<Output = Result<(), Self::Error>>;

        type UpdateFuture<R>: Future<Output = Result<(), EitherError<Self::Error, R::Error>>>
        where
            R: Read;

        fn complete(self) -> Self::CompleteFuture;

        fn abort(self) -> Self::AbortFuture;

        fn update<R>(self, read: R, progress: impl Fn(u64, u64)) -> Self::UpdateFuture<R>
        where
            R: Read,
            Self: Sized;
    }

    pub trait OtaRead: io::asynch::Read {
        fn size(&self) -> Option<usize>;
    }

    pub trait OtaServer: Io {
        type OtaRead<'a>: OtaRead<Error = Self::Error>
        where
            Self: 'a;

        type GetLatestReleaseFuture<'a>: Future<Output = Result<Option<FirmwareInfo>, Self::Error>>
        where
            Self: 'a;

        #[cfg(feature = "alloc")]
        type GetReleasesFuture<'a>: Future<
            Output = Result<alloc::vec::Vec<FirmwareInfo>, Self::Error>,
        >
        where
            Self: 'a;

        type GetReleasesNFuture<'a, const N: usize>: Future<
            Output = Result<heapless::Vec<FirmwareInfo, N>, Self::Error>,
        >
        where
            Self: 'a;

        type OpenFuture<'a>: Future<Output = Result<Self::OtaRead<'a>, Self::Error>>
        where
            Self: 'a;

        fn get_latest_release(&mut self) -> Self::GetLatestReleaseFuture<'_>;

        #[cfg(feature = "alloc")]
        fn get_releases(&mut self) -> Self::GetReleasesFuture<'_>;

        fn get_releases_n<const N: usize>(&mut self) -> Self::GetReleasesNFuture<'_, N>;

        fn open<'a>(&'a mut self, download_id: &'a str) -> Self::OpenFuture<'a>;
    }
}
