#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

use crate::io::{ErrorType, Read, Write};
use crate::utils::io::*;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct Slot {
    pub label: heapless::String<32>,
    pub state: SlotState,
    pub firmware: Option<FirmwareInfo>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct FirmwareInfo {
    pub version: heapless::String<24>,
    pub released: heapless::String<24>,
    pub description: Option<heapless::String<128>>,
    pub signature: Option<heapless::Vec<u8, 32>>,
    pub download_id: Option<heapless::String<128>>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub struct UpdateProgress {
    pub progress: u32,
    pub operation: &'static str,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum LoadResult {
    ReloadMore,
    LoadMore,
    Loaded,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Hash))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
pub enum SlotState {
    Factory,
    Valid,
    Invalid,
    Unverified,
    Unknown,
}

pub trait FirmwareInfoLoader: ErrorType {
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

pub trait Ota: ErrorType {
    type Update<'a>: OtaUpdate<Error = Self::Error>
    where
        Self: 'a;

    fn get_boot_slot(&self) -> Result<Slot, Self::Error>;

    fn get_running_slot(&self) -> Result<Slot, Self::Error>;

    fn get_update_slot(&self) -> Result<Slot, Self::Error>;

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
    type Update<'a> = O::Update<'a> where Self: 'a;

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
    type OtaUpdateFinished: OtaUpdateFinished;

    fn finish(self) -> Result<Self::OtaUpdateFinished, Self::Error>;

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

pub trait OtaUpdateFinished: ErrorType {
    fn activate(self) -> Result<(), Self::Error>;
}

pub mod asynch {
    use crate::io::asynch::{ErrorType, Read, Write};
    use crate::utils::io::asynch::*;

    pub use super::{FirmwareInfo, FirmwareInfoLoader, LoadResult, Slot, SlotState};

    pub trait Ota: ErrorType {
        type Update<'a>: OtaUpdate<Error = Self::Error>
        where
            Self: 'a;

        async fn get_boot_slot(&self) -> Result<Slot, Self::Error>;

        async fn get_running_slot(&self) -> Result<Slot, Self::Error>;

        async fn get_update_slot(&self) -> Result<Slot, Self::Error>;

        async fn is_factory_reset_supported(&self) -> Result<bool, Self::Error>;

        async fn factory_reset(&mut self) -> Result<(), Self::Error>;

        async fn initiate_update(&mut self) -> Result<Self::Update<'_>, Self::Error>;

        async fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error>;

        async fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error;
    }

    impl<O> Ota for &mut O
    where
        O: Ota,
    {
        type Update<'a> = O::Update<'a> where Self: 'a;

        async fn get_boot_slot(&self) -> Result<Slot, Self::Error> {
            (**self).get_boot_slot().await
        }

        async fn get_running_slot(&self) -> Result<Slot, Self::Error> {
            (**self).get_running_slot().await
        }

        async fn get_update_slot(&self) -> Result<Slot, Self::Error> {
            (**self).get_update_slot().await
        }

        async fn is_factory_reset_supported(&self) -> Result<bool, Self::Error> {
            (**self).is_factory_reset_supported().await
        }

        async fn factory_reset(&mut self) -> Result<(), Self::Error> {
            (*self).factory_reset().await
        }

        async fn initiate_update(&mut self) -> Result<Self::Update<'_>, Self::Error> {
            (*self).initiate_update().await
        }

        async fn mark_running_slot_valid(&mut self) -> Result<(), Self::Error> {
            (*self).mark_running_slot_valid().await
        }

        async fn mark_running_slot_invalid_and_reboot(&mut self) -> Self::Error {
            (*self).mark_running_slot_invalid_and_reboot().await
        }
    }

    pub trait OtaUpdate: Write {
        type OtaUpdateFinished: OtaUpdateFinished;

        async fn finish(self) -> Result<Self::OtaUpdateFinished, Self::Error>;

        async fn complete(self) -> Result<(), Self::Error>;

        async fn abort(self) -> Result<(), Self::Error>;

        async fn update<R>(
            self,
            read: R,
            progress: impl Fn(u64, u64),
        ) -> Result<(), CopyError<R::Error, Self::Error>>
        where
            R: Read,
            Self: Sized;
    }

    pub trait OtaUpdateFinished: ErrorType {
        async fn activate(self) -> Result<(), Self::Error>;
    }
}
