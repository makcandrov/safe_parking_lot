#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

use std::ops::Deref;

use parking_lot::{RwLock, RwLockWriteGuard};

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

impl<'a, T> SafeRwLock<'a, T> {
    /// Creates a new `SafeRwLock` referencing the given `RwLock`.
    ///
    /// This does not lock the underlying `RwLock`.
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

    /// Releases the lock and returns the original `SafeRwLock`.
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
