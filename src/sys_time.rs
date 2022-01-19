use core::time::Duration;

pub trait SystemTime {
    fn now(&self) -> Duration;
}
