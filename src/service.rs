use core::fmt::{Debug, Display};

pub trait Service {
    type Error: Display + Debug + Send + Sync + 'static;
}
