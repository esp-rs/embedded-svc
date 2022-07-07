use core::time::Duration;

pub trait SystemTime {
    fn now(&self) -> Duration;
}

impl<S> SystemTime for &S
where
    S: SystemTime,
{
    fn now(&self) -> Duration {
        (*self).now()
    }
}
