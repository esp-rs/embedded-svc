#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::io::{Io, Read, Write};
use crate::utils::io::*;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Slot {
    pub label: heapless::String<32>,
    pub state: SlotState,
    pub firmware: Option<FirmwareInfo>,
}

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
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum LoadResult {
    ReloadMore,
    LoadMore,
    Loaded,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum SlotState {
    Factory,
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

pub trait Ota: Io {
    type Update: OtaUpdate<Error = Self::Error>;

    fn get_boot_slot(&self) -> Result<Slot, Self::Error>;

    fn get_running_slot(&self) -> Result<Slot, Self::Error>;

    fn get_update_slot(&self) -> Result<Slot, Self::Error>;

    fn is_factory_reset_supported(&self) -> Result<bool, Self::Error>;

    fn factory_reset(&mut self) -> Result<(), Self::Error>;

    fn initiate_update(&mut self) -> Result<&mut Self::Update, Self::Error>;

    fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error>;

    fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
}

impl<O> Ota for &mut O
where
    O: Ota,
{
    type Update = O::Update;

    fn get_boot_slot(&self) -> Result<Slot, Self::Error> {
        (**self).get_boot_slot()
    }

    fn get_running_slot(&self) -> Result<Slot, Self::Error> {
        (**self).get_running_slot()
    }

    fn get_update_slot(&self) -> Result<Slot, Self::Error> {
        (**self).get_update_slot()
    }

    fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
        (**self).is_factory_reset_supported()
    }

    fn factory_reset(&mut self) -> Result<(), Self::Error> {
        (*self).factory_reset()
    }

    fn initiate_update(&mut self) -> Result<&mut Self::Update, Self::Error> {
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
    fn complete(&mut self) -> Result<(), Self::Error>;

    fn abort(&mut self) -> Result<(), Self::Error>;

    fn update<R>(
        &mut self,
        read: R,
        progress: impl Fn(u64, u64),
    ) -> Result<(), CopyError<R::Error, Self::Error>>
    where
        R: Read,
        Self: Sized,
    {
        let mut buf = [0_u8; 64];

        match copy_len_with_progress(read, &mut *self, &mut buf, u64::MAX, progress) {
            Ok(_) => self.complete().map_err(CopyError::Write),
            Err(e) => {
                self.abort().map_err(CopyError::Write)?;

                Err(e)
            }
        }
    }
}

#[cfg(all(feature = "nightly", feature = "experimental"))]
pub mod asynch {
    use core::future::Future;

    use crate::executor::asynch::{Blocker, RawBlocking};
    use crate::io::asynch::{Io, Read, Write};
    use crate::utils::io::asynch::*;

    pub use super::{FirmwareInfo, FirmwareInfoLoader, LoadResult, Slot, SlotState};

    pub trait Ota: Io {
        type Update: OtaUpdate<Error = Self::Error>;

        type GetBootSlotFuture<'a>: Future<Output = Result<Slot, Self::Error>>
        where
            Self: 'a;

        type GetRunningSlotFuture<'a>: Future<Output = Result<Slot, Self::Error>>
        where
            Self: 'a;

        type GetUpdateSlotFuture<'a>: Future<Output = Result<Slot, Self::Error>>
        where
            Self: 'a;

        type FactoryResetFuture<'a>: Future<Output = Result<(), Self::Error>>
        where
            Self: 'a;

        type InitiateUpdateFuture<'a>: Future<Output = Result<&'a mut Self::Update, Self::Error>>
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
        type Update = O::Update;

        type GetBootSlotFuture<'a> = O::GetBootSlotFuture<'a> where Self: 'a;

        type GetRunningSlotFuture<'a> = O::GetRunningSlotFuture<'a> where Self: 'a;

        type GetUpdateSlotFuture<'a> = O::GetUpdateSlotFuture<'a> where Self: 'a;

        type FactoryResetFuture<'a> = O::FactoryResetFuture<'a> where Self: 'a;

        type InitiateUpdateFuture<'a> = O::InitiateUpdateFuture<'a> where Self: 'a;

        type MarkRunningSlotValidFuture<'a> = O::MarkRunningSlotValidFuture<'a> where Self: 'a;

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

        fn complete(&mut self) -> Self::CompleteFuture;

        fn abort(&mut self) -> Self::AbortFuture;

        fn update<R>(&mut self, read: R, progress: impl Fn(u64, u64)) -> Self::UpdateFuture<R>
        where
            R: Read,
            Self: Sized;
    }

    #[derive(Debug)]
    pub struct BlockingOta<B, O>
    where
        O: Ota,
    {
        blocker: B,
        ota: O,
        lended_update: RawBlocking<B, O::Update>,
    }

    impl<B, O> BlockingOta<B, O>
    where
        B: Blocker,
        O: Ota,
    {
        pub const fn new(blocker: B, ota: O) -> Self {
            Self {
                blocker,
                ota,
                lended_update: RawBlocking::new(),
            }
        }
    }

    impl<B, O> Io for BlockingOta<B, O>
    where
        O: Ota,
    {
        type Error = O::Error;
    }

    impl<B, O> super::Ota for BlockingOta<B, O>
    where
        B: Blocker,
        O: Ota,
    {
        type Update = RawBlocking<B, O::Update>;

        fn get_boot_slot(&self) -> Result<Slot, Self::Error> {
            self.blocker.block_on(self.ota.get_boot_slot())
        }

        fn get_running_slot(&self) -> Result<Slot, Self::Error> {
            self.blocker.block_on(self.ota.get_running_slot())
        }

        fn get_update_slot(&self) -> Result<Slot, Self::Error> {
            self.blocker.block_on(self.ota.get_update_slot())
        }

        fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
            self.ota.is_factory_reset_supported()
        }

        fn factory_reset(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.ota.factory_reset())
        }

        fn initiate_update(&mut self) -> Result<&mut Self::Update, Self::Error> {
            let update = self.blocker.block_on(self.ota.initiate_update())?;

            self.lended_update.blocker = &self.blocker;
            self.lended_update.api = update;

            Ok(&mut self.lended_update)
        }

        fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error> {
            self.blocker.block_on(self.ota.mark_running_slot_valid())
        }

        fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error {
            self.ota.mark_running_slot_invalid_and_reboot()
        }
    }

    impl<B, U> super::OtaUpdate for RawBlocking<B, U>
    where
        B: Blocker,
        U: OtaUpdate,
    {
        fn complete(&mut self) -> Result<(), Self::Error> {
            unsafe { self.blocker.as_ref().unwrap() }
                .block_on(unsafe { self.api.as_mut() }.unwrap().complete())
        }

        fn abort(&mut self) -> Result<(), Self::Error> {
            unsafe { self.blocker.as_ref().unwrap() }
                .block_on(unsafe { self.api.as_mut() }.unwrap().abort())
        }
    }
}
