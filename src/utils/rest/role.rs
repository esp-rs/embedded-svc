use enumset::*;

#[cfg(feature = "use_serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "use_strum")]
use strum_macros::{EnumIter, EnumMessage, EnumString, Display};

#[cfg(feature = "use_numenum")]
use num_enum::TryFromPrimitive;

pub const ADMIN_USERNAME: &str = "admin";

#[derive(EnumSetType, Debug, PartialOrd)]
#[cfg_attr(feature = "use_serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "use_strum",
    derive(EnumString, Display, EnumMessage, EnumIter)
)]
#[cfg_attr(feature = "use_numenum", derive(TryFromPrimitive))]
#[cfg_attr(feature = "use_numenum", repr(u8))]
pub enum Role {
    #[cfg_attr(feature = "use_strum", strum(serialize = "none", message = "None"))]
    None,

    #[cfg_attr(feature = "use_strum", strum(serialize = "user", message = "User"))]
    User,

    #[cfg_attr(feature = "use_strum", strum(serialize = "admin", message = "Admin"))]
    Admin,
}

impl Default for Role {
    fn default() -> Self {
        Role::None
    }
}
