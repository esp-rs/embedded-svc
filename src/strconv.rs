#[derive(Debug)]
pub struct StrConvError;

impl core::fmt::Display for StrConvError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "StrConvError")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StrConvError {}
