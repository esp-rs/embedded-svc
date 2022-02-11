pub mod event_bus;
pub mod mqtt;
pub mod timer;

pub trait AsyncWrapper<S> {
    fn new(sync: S) -> Self;
}

pub trait Asyncify {
    type AsyncWrapper<S>: AsyncWrapper<S>;

    fn into_async(self) -> Self::AsyncWrapper<Self>
    where
        Self: Sized,
    {
        Self::AsyncWrapper::new(self)
    }

    fn as_async(&mut self) -> Self::AsyncWrapper<&mut Self> {
        Self::AsyncWrapper::new(self)
    }
}
