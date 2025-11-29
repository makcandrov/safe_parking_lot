#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

use std::ops::Deref;

use parking_lot::{MappedRwLockWriteGuard, RwLock, RwLockWriteGuard};

/// A borrow-checker–friendly wrapper around `parking_lot::RwLock`.
///
/// `SafeRwLock` represents access to an `RwLock` in a way that enforces:
///
/// - You may lock and inspect the value.
/// - You may not mutate until you explicitly request a writable guard.
/// - Unlocking returns you to a clean state that must be re-locked.
///
/// This helps ensure that conditional write logic is expressed correctly,
/// especially inside loops or retry patterns.
#[derive(Debug)]
pub struct SafeRwLock<'a, T>(&'a RwLock<T>);

/// A temporary write-lock holder that allows **inspection only**.
///
/// After calling [`SafeRwLock::lock`], you receive a `SafeRwLockGuard`.
/// This guard allows read-only access to the data. To proceed, you must
/// choose one of two actions:
///
/// - [`unlock`](Self::unlock): release the lock and regain a `SafeRwLock`, or
/// - [`upgrade`](Self::upgrade): convert into a full `RwLockWriteGuard`
///   allowing mutation.
///
/// By separating inspection from mutation, the compiler can enforce that
/// no modification happens before you explicitly upgrade.
#[derive(Debug)]
pub struct SafeRwLockGuard<'a, T> {
    lock: SafeRwLock<'a, T>,
    guard: RwLockWriteGuard<'a, T>,
}

/// A temporary write-lock holder that allows **inspection only** on a mapped view of the data.
///
/// After calling [`SafeRwLockGuard::map`], you receive a `SafeMappedRwLockGuard`.
/// This guard allows read-only access to a mapped view of the data. To proceed, you must
/// choose one of two actions:
///
/// - [`unlock`](Self::unlock): release the lock and regain a `SafeRwLock`, or
/// - [`upgrade`](Self::upgrade): convert into a full `MappedRwLockWriteGuard`
///   allowing mutation of the mapped data.
///
/// By separating inspection from mutation, the compiler can enforce that no modification
/// happens before you explicitly upgrade.
#[derive(Debug)]
pub struct SafeMappedRwLockGuard<'a, T, U> {
    lock: SafeRwLock<'a, T>,
    guard: MappedRwLockWriteGuard<'a, U>,
}

impl<'a, T> SafeRwLock<'a, T> {
    /// Creates a new [`SafeRwLock`](SafeRwLock) referencing the given [`RwLock`](RwLock).
    ///
    /// This does not lock the underlying [`RwLock`](RwLock).
    pub fn new(lock: &'a RwLock<T>) -> Self {
        Self(lock)
    }

    /// Acquires a write lock and returns a guard that allows inspection
    /// but not yet mutation.
    ///
    /// To modify the value, you must call [`SafeRwLockGuard::upgrade`].
    pub fn lock(self) -> SafeRwLockGuard<'a, T> {
        SafeRwLockGuard {
            guard: self.0.write(),
            lock: self,
        }
    }

    /// Attempts to acquire a write lock.
    ///
    /// Returns:
    /// - `Ok(SafeRwLockGuard)` if successful,
    /// - `Err(self)` if the lock is currently held by another thread.
    pub fn try_lock(self) -> Result<SafeRwLockGuard<'a, T>, Self> {
        match self.0.try_write() {
            Some(guard) => Ok(SafeRwLockGuard { guard, lock: self }),
            None => Err(self),
        }
    }
}

impl<'a, T> SafeRwLockGuard<'a, T> {
    /// Converts this guard into a real write guard, enabling mutation.
    ///
    /// After upgrading, you work directly with the underlying
    /// `RwLockWriteGuard`, and normal drop semantics apply.
    pub fn upgrade(self) -> RwLockWriteGuard<'a, T> {
        self.guard
    }

    /// Releases the lock and returns the original [`SafeRwLock`](SafeRwLock).
    ///
    /// This is typically used when a condition is not met and you want to
    /// retry locking later without performing any mutation.
    pub fn unlock(self) -> SafeRwLock<'a, T> {
        self.lock
    }

    /// Maps the guarded value to a different type and returns a new guard for that type.
    ///
    /// This function allows you to create a mapped view of the data protected by the lock.
    /// You can then access the mapped data immutably. To mutate it, you would need to call
    /// the [`upgrade`](Self::upgrade) method to convert to a full write guard.
    pub fn map<U, F>(self, f: F) -> SafeMappedRwLockGuard<'a, T, U>
    where
        F: FnOnce(&mut T) -> &mut U,
    {
        SafeMappedRwLockGuard {
            lock: self.lock,
            guard: RwLockWriteGuard::map(self.guard, f),
        }
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
        match RwLockWriteGuard::try_map(self.guard, f) {
            Ok(guard) => Ok(SafeMappedRwLockGuard {
                guard,
                lock: self.lock,
            }),
            Err(guard) => Err(Self {
                guard,
                lock: self.lock,
            }),
        }
    }
}

impl<'a, T, U> SafeMappedRwLockGuard<'a, T, U> {
    /// Converts this guard into a real write guard, enabling mutation.
    ///
    /// After upgrading, you work directly with the underlying
    /// `MappedRwLockWriteGuard`, and normal drop semantics apply.
    pub fn upgrade(self) -> MappedRwLockWriteGuard<'a, U> {
        self.guard
    }

    /// Releases the lock and returns the original [`SafeRwLock`](SafeRwLock).
    ///
    /// This is typically used when a condition is not met and you want to
    /// retry locking later without performing any mutation.
    pub fn unlock(self) -> SafeRwLock<'a, T> {
        self.lock
    }
}

impl<'a, T> Deref for SafeRwLockGuard<'a, T> {
    type Target = T;

    /// Provides read-only access to the underlying value.
    ///
    /// Mutation is only possible after calling [`upgrade`](Self::upgrade).
    fn deref(&self) -> &Self::Target {
        Deref::deref(&self.guard)
    }
}

impl<'a, T, U> Deref for SafeMappedRwLockGuard<'a, T, U> {
    type Target = U;

    /// Provides read-only access to the underlying value.
    ///
    /// Mutation is only possible after calling [`upgrade`](Self::upgrade).
    fn deref(&self) -> &Self::Target {
        Deref::deref(&self.guard)
    }
}
