use ::std::sync::{PoisonError, RwLock, RwLockWriteGuard, TryLockError};

use crate::{LockBlocking, LockImmediate, SafeGuard, SafeLock};

pub type SafeRwLock<'a, T> = SafeLock<&'a RwLock<T>>;
pub type SafeRwLockGuard<'a, T> = SafeGuard<&'a RwLock<T>, RwLockWriteGuard<'a, T>>;

impl<'a, T> LockBlocking for &'a RwLock<T> {
    type Error = PoisonError<RwLockWriteGuard<'a, T>>;
    type Guard = RwLockWriteGuard<'a, T>;

    fn lock_blocking(&self) -> Result<Self::Guard, Self::Error> {
        self.write()
    }
}

impl<'a, T> LockImmediate for &'a RwLock<T> {
    type Error = TryLockError<RwLockWriteGuard<'a, T>>;
    type Guard = RwLockWriteGuard<'a, T>;

    fn lock_immediate(&self) -> Result<Self::Guard, Self::Error> {
        self.try_write()
    }
}
