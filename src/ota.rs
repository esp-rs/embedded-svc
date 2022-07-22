#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::io::{self, Error, Io, Read, Write};
use crate::utils::io::*;

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

impl<F> FirmwareInfoLoader for &mut F
where
    F: FirmwareInfoLoader,
{
    fn load(&mut self, buf: &[u8]) -> Result<LoadResult, Self::Error> {
        (*self).load(buf)
    }

    fn is_loaded(&self) -> bool {
        (**self).is_loaded()
    }

    fn get_info(&self) -> Result<FirmwareInfo, Self::Error> {
        (**self).get_info()
    }
}

pub trait OtaSlot {
    type Error: Error;

    fn get_label(&self) -> Result<&str, Self::Error>;

    fn get_state(&self) -> Result<SlotState, Self::Error>;

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error>;
}

impl<O> OtaSlot for &O
where
    O: OtaSlot,
{
    type Error = O::Error;

    fn get_label(&self) -> Result<&str, Self::Error> {
        (*self).get_label()
    }

    fn get_state(&self) -> Result<SlotState, Self::Error> {
        (*self).get_state()
    }

    fn get_firmware_info(&self) -> Result<Option<FirmwareInfo>, Self::Error> {
        (*self).get_firmware_info()
    }
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

impl<O> Ota for &mut O
where
    O: Ota,
{
    type Slot<'a>
    where
        Self: 'a,
    = O::Slot<'a>;

    type Update<'a>
    where
        Self: 'a,
    = O::Update<'a>;

    fn get_boot_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
        (**self).get_boot_slot()
    }

    fn get_running_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
        (**self).get_running_slot()
    }

    fn get_update_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
        (**self).get_update_slot()
    }

    fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
        (**self).is_factory_reset_supported()
    }

    fn factory_reset(&mut self) -> Result<(), Self::Error> {
        (*self).factory_reset()
    }

    fn initiate_update(&mut self) -> Result<Self::Update<'_>, Self::Error> {
        (*self).initiate_update()
    }

    fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error> {
        (*self).mark_running_slot_valid()
    }

    fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error {
        (*self).mark_running_slot_invalid_and_reboot()
    }
}

pub trait OtaUpdate: Write {
    fn complete(self) -> Result<(), Self::Error>;

    fn abort(self) -> Result<(), Self::Error>;

    fn update<R>(
        mut self,
        read: R,
        progress: impl Fn(u64, u64),
    ) -> Result<(), CopyError<R::Error, Self::Error>>
    where
        R: Read,
        Self: Sized,
    {
        let mut buf = [0_u8; 64];

        match copy_len_with_progress(read, &mut self, &mut buf, u64::MAX, progress) {
            Ok(_) => self.complete().map_err(CopyError::Write),
            Err(e) => {
                self.abort().map_err(CopyError::Write)?;

                Err(e)
            }
        }
    }
}

pub trait OtaRead: io::Read {
    fn size(&self) -> Option<usize>;
}

impl<R> OtaRead for &mut R
where
    R: OtaRead,
{
    fn size(&self) -> Option<usize> {
        (**self).size()
    }
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

    fn open<'a>(&'a mut self, download_id: &'a str) -> Result<Self::OtaRead<'a>, Self::Error>;
}

impl<O> OtaServer for &mut O
where
    O: OtaServer,
{
    type OtaRead<'a>
    where
        Self: 'a,
    = O::OtaRead<'a>;

    fn get_latest_release(&mut self) -> Result<Option<FirmwareInfo>, Self::Error> {
        (*self).get_latest_release()
    }

    fn get_releases(&mut self) -> Result<alloc::vec::Vec<FirmwareInfo>, Self::Error> {
        (*self).get_releases()
    }

    fn get_releases_n<const N: usize>(
        &mut self,
    ) -> Result<heapless::Vec<FirmwareInfo, N>, Self::Error> {
        (*self).get_releases_n()
    }

    fn open<'a>(&'a mut self, download_id: &'a str) -> Result<Self::OtaRead<'a>, Self::Error> {
        (*self).open(download_id)
    }
}

#[cfg(feature = "experimental")]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, Blocking};
    use crate::io::asynch::{Io, Read, Write};
    use crate::utils::io::asynch::*;

    pub use super::{FirmwareInfo, FirmwareInfoLoader, LoadResult, OtaSlot, SlotState};

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

    impl<O> Ota for &mut O
    where
        O: Ota,
    {
        type Slot<'a>
        where
            Self: 'a,
        = O::Slot<'a>;

        type Update<'a>
        where
            Self: 'a,
        = O::Update<'a>;

        type GetBootSlotFuture<'a>
        where
            Self: 'a,
        = O::GetBootSlotFuture<'a>;

        type GetRunningSlotFuture<'a>
        where
            Self: 'a,
        = O::GetRunningSlotFuture<'a>;

        type GetUpdateSlotFuture<'a>
        where
            Self: 'a,
        = O::GetUpdateSlotFuture<'a>;

        type FactoryResetFuture<'a>
        where
            Self: 'a,
        = O::FactoryResetFuture<'a>;

        type InitiateUpdateFuture<'a>
        where
            Self: 'a,
        = O::InitiateUpdateFuture<'a>;

        type MarkRunningSlotValidFuture<'a>
        where
            Self: 'a,
        = O::MarkRunningSlotValidFuture<'a>;

        fn get_boot_slot(&self) -> Self::GetBootSlotFuture<'_> {
            (**self).get_boot_slot()
        }

        fn get_running_slot(&self) -> Self::GetRunningSlotFuture<'_> {
            (**self).get_running_slot()
        }

        fn get_update_slot(&self) -> Self::GetUpdateSlotFuture<'_> {
            (**self).get_update_slot()
        }

        fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
            (**self).is_factory_reset_supported()
        }

        fn factory_reset(&mut self) -> Self::FactoryResetFuture<'_> {
            (*self).factory_reset()
        }

        fn initiate_update(&mut self) -> Self::InitiateUpdateFuture<'_> {
            (*self).initiate_update()
        }

        fn mark_running_slot_valid(&mut self) -> Self::MarkRunningSlotValidFuture<'_> {
            (*self).mark_running_slot_valid()
        }

        fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error {
            (*self).mark_running_slot_invalid_and_reboot()
        }
    }

    pub trait OtaUpdate: Write {
        type CompleteFuture: Future<Output = Result<(), Self::Error>>;

        type AbortFuture: Future<Output = Result<(), Self::Error>>;

        type UpdateFuture<R>: Future<Output = Result<(), CopyError<R::Error, Self::Error>>>
        where
            R: Read;

        fn complete(self) -> Self::CompleteFuture;

        fn abort(self) -> Self::AbortFuture;

        fn update<R>(self, read: R, progress: impl Fn(u64, u64)) -> Self::UpdateFuture<R>
        where
            R: Read,
            Self: Sized;
    }

    pub trait OtaRead: Read {
        fn size(&self) -> Option<usize>;
    }

    impl<R> OtaRead for &mut R
    where
        R: OtaRead,
    {
        fn size(&self) -> Option<usize> {
            (**self).size()
        }
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

    impl<O> OtaServer for &mut O
    where
        O: OtaServer,
    {
        type OtaRead<'a>
        where
            Self: 'a,
        = O::OtaRead<'a>;

        type GetLatestReleaseFuture<'a>
        where
            Self: 'a,
        = O::GetLatestReleaseFuture<'a>;

        type GetReleasesFuture<'a>
        where
            Self: 'a,
        = O::GetReleasesFuture<'a>;

        type GetReleasesNFuture<'a, const N: usize>
        where
            Self: 'a,
        = O::GetReleasesNFuture<'a, N>;

        type OpenFuture<'a>
        where
            Self: 'a,
        = O::OpenFuture<'a>;

        fn get_latest_release(&mut self) -> Self::GetLatestReleaseFuture<'_> {
            (*self).get_latest_release()
        }

        fn get_releases(&mut self) -> Self::GetReleasesFuture<'_> {
            (*self).get_releases()
        }

        fn get_releases_n<const N: usize>(&mut self) -> Self::GetReleasesNFuture<'_, N> {
            (*self).get_releases_n()
        }

        fn open<'a>(&'a mut self, download_id: &'a str) -> Self::OpenFuture<'a> {
            (*self).open(download_id)
        }
    }

    impl<B, O> super::Ota for Blocking<B, O>
    where
        B: Blocker,
        O: Ota,
    {
        type Slot<'a>
        where
            Self: 'a,
        = O::Slot<'a>;

        type Update<'a>
        where
            Self: 'a,
        = Blocking<&'a B, O::Update<'a>>;

        fn get_boot_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
            self.blocker.block_on(self.api.get_boot_slot())
        }

        fn get_running_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
            self.blocker.block_on(self.api.get_running_slot())
        }

        fn get_update_slot(&self) -> Result<Self::Slot<'_>, Self::Error> {
            self.blocker.block_on(self.api.get_update_slot())
        }

        fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
            self.api.is_factory_reset_supported()
        }

        fn factory_reset(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.factory_reset())
        }

        fn initiate_update(&mut self) -> Result<Self::Update<'_>, Self::Error> {
            let update = self.blocker.block_on(self.api.initiate_update())?;

            Ok(Blocking::new(&self.blocker, update))
        }

        fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.mark_running_slot_valid())
        }

        fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error {
            self.api.mark_running_slot_invalid_and_reboot()
        }
    }

    impl<B, U> super::OtaUpdate for Blocking<B, U>
    where
        B: Blocker,
        U: OtaUpdate,
    {
        fn complete(self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.complete())
        }

        fn abort(self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.api.abort())
        }
    }
}
