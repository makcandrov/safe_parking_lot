use ::core::convert::Infallible;

use parking_lot::{MappedRwLockWriteGuard, RwLock, RwLockWriteGuard};

use crate::{LockBlocking, LockImmediate, SafeGuard, SafeLock};

/// A wrapper around [`RwLock`](RwLock) from `parking_lot`, providing safe locking behavior.
pub type SafeRwLock<'a, T> = SafeLock<&'a RwLock<T>>;
pub type SafeRwLockGuard<'a, T> = SafeGuard<&'a RwLock<T>, RwLockWriteGuard<'a, T>>;
pub type SafeMappedRwLockGuard<'a, T, U> = SafeGuard<&'a RwLock<T>, MappedRwLockWriteGuard<'a, U>>;

impl<'a, T> LockBlocking for &'a RwLock<T> {
    type Error = Infallible;
    type Guard = RwLockWriteGuard<'a, T>;

    fn lock_blocking(&self) -> Result<Self::Guard, Self::Error> {
        Ok(self.write())
    }
}

impl<'a, T> LockImmediate for &'a RwLock<T> {
    type Error = ();
    type Guard = RwLockWriteGuard<'a, T>;

    fn lock_immediate(&self) -> Result<Self::Guard, Self::Error> {
        self.try_write().ok_or(())
    }
}

impl<'a, T> SafeRwLockGuard<'a, T> {
    /// Maps the guarded value to a different type and returns a new guard for that type.
    ///
    /// This function allows you to create a mapped view of the data protected by the lock.
    /// You can then access the mapped data immutably. To mutate it, you would need to call
    /// the [`upgrade`](Self::upgrade) method to convert to a full write guard.
    pub fn map<U, F>(self, f: F) -> SafeMappedRwLockGuard<'a, T, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        self.map_guard(|guard| RwLockWriteGuard::map(guard, f))
    }

    /// Attempts to map the guarded value to a different type, returning a guard for the mapped data.
    ///
    /// This method works similarly to `map`, but with an additional check: it attempts to map the
    /// value only if the mapping function returns `Some`. If the mapping function returns `None`,
    /// the operation fails, and no mapping occurs. This provides more control when mapping is conditional.
    pub fn try_map<U, F>(self, f: F) -> Result<SafeMappedRwLockGuard<'a, T, U>, Self>
    where
        F: FnOnce(&mut T) -> Option<&mut U>,
    {
        self.try_map_guard(|guard| RwLockWriteGuard::try_map(guard, f))
    }
}
