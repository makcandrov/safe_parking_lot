#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

use ::core::{convert::Infallible, ops::Deref};

#[cfg(feature = "parking_lot")]
pub mod parking_lot;
#[cfg(feature = "std")]
pub mod std;

/// A wrapper around a lock type `L` that ensures safe locking behavior.
///
/// The `SafeLock` type provides methods for acquiring and releasing locks while
/// enforcing safety against common locking mistakes like attempting to mutate data
/// before confirming conditions or improperly handling the lock state across retries.
#[derive(Debug)]
pub struct SafeLock<L>(L);

/// A guard for a lock type `L` that holds the lock and allows inspection of the
/// data through a guard type `G`. The guard prevents mutation until explicitly upgraded.
///
/// This guard ensures that mutation is performed only after explicitly upgrading the lock.
#[derive(Debug)]
pub struct SafeGuard<L, G> {
    lock: SafeLock<L>,
    guard: G,
}

/// Trait for locks that support blocking behavior.
///
/// This trait provides a method to acquire the lock in a blocking manner and returns
/// a guard to access the locked data. It is intended for types of locks that block
/// the current thread until the lock becomes available.
pub trait LockBlocking {
    type Error;
    type Guard;

    /// Blocks the current thread until the lock can be acquired.
    ///
    /// Returns a guard that allows access to the data protected by the lock.
    fn lock_blocking(&self) -> Result<Self::Guard, Self::Error>;
}

/// Trait for locks that support immediate locking without blocking.
///
/// This trait provides a method to try to acquire the lock without blocking.
pub trait LockImmediate {
    type Error;
    type Guard;

    /// Attempts to acquire the lock immediately, without blocking.
    ///
    /// Returns a guard if successful, or an error if the lock is unavailable.
    fn lock_immediate(&self) -> Result<Self::Guard, Self::Error>;
}

impl<L> SafeLock<L> {
    /// Creates a new [`SafeLock`] wrapping the provided lock.
    ///
    /// This does not acquire the lock; it only wraps the lock into a [`SafeLock`] type.
    pub const fn new(lock: L) -> Self {
        Self(lock)
    }

    /// Acquires the lock in write mode and returns a guard for the locked data.
    ///
    /// The lock is acquired in **write mode**, and the returned guard allows read-only access to the data.
    /// Mutation is not possible until explicitly upgrading the guard.
    pub fn lock_blocking(self) -> SafeGuard<L, L::Guard>
    where
        L: LockBlocking<Error = Infallible>,
    {
        SafeGuard {
            guard: LockBlocking::lock_blocking(&self.0).unwrap(),
            lock: self,
        }
    }

    /// Attempts to acquire the lock in write mode and returns a guard if successful.
    ///
    /// The lock is acquired in **write mode**. If the lock is already held, this method will return `Err(self)`.
    pub fn try_lock_blocking(self) -> Result<SafeGuard<L, L::Guard>, Self>
    where
        L: LockBlocking,
    {
        match LockBlocking::lock_blocking(&self.0) {
            Ok(guard) => Ok(SafeGuard { lock: self, guard }),
            Err(_) => Err(self),
        }
    }

    /// Attempts to acquire the lock in write mode and returns an error if it fails.
    ///
    /// The lock is acquired in **write mode**, and the method returns an error if the lock is unavailable.
    pub fn try_lock_blocking_err(self) -> Result<SafeGuard<L, L::Guard>, (Self, L::Error)>
    where
        L: LockBlocking,
    {
        match LockBlocking::lock_blocking(&self.0) {
            Ok(guard) => Ok(SafeGuard { lock: self, guard }),
            Err(err) => Err((self, err)),
        }
    }

    /// Acquires the lock in write mode without blocking and returns a guard.
    ///
    /// This method tries to acquire the lock in **write mode** without blocking the current thread.
    /// The lock is either acquired successfully or the method returns an error.
    pub fn lock_immediate(self) -> SafeGuard<L, L::Guard>
    where
        L: LockImmediate<Error = Infallible>,
    {
        SafeGuard {
            guard: LockImmediate::lock_immediate(&self.0).unwrap(),
            lock: self,
        }
    }

    /// Attempts to acquire the lock in write mode without blocking and returns a guard if successful.
    ///
    /// If the lock is already held, this method will return `Err(self)` without blocking.
    pub fn try_lock_immediate(self) -> Result<SafeGuard<L, L::Guard>, Self>
    where
        L: LockImmediate,
    {
        match LockImmediate::lock_immediate(&self.0) {
            Ok(guard) => Ok(SafeGuard { lock: self, guard }),
            Err(_) => Err(self),
        }
    }

    /// Attempts to acquire the lock in write mode immediately and returns an error if unsuccessful.
    pub fn try_lock_immediate_err(self) -> Result<SafeGuard<L, L::Guard>, (Self, L::Error)>
    where
        L: LockImmediate,
    {
        match LockImmediate::lock_immediate(&self.0) {
            Ok(guard) => Ok(SafeGuard { lock: self, guard }),
            Err(err) => Err((self, err)),
        }
    }
}

impl<L, G> SafeGuard<L, G> {
    /// Upgrades the `SafeGuard` to the underlying guard, allowing mutation of the locked data.
    ///
    /// Even though the underlying lock was acquired in **write mode**, mutation of the data is only
    /// possible after explicitly upgrading the guard. This ensures safety in concurrent code.
    pub fn upgrade(self) -> G {
        self.guard
    }

    /// Releases the lock and returns the original [`SafeLock`], allowing further locking attempts.
    ///
    /// This method is useful when retrying to acquire the lock under certain conditions.
    pub fn unlock(self) -> SafeLock<L> {
        self.lock
    }

    /// Maps the guarded value to a different type, returning a new guard for the mapped data.
    ///
    /// The function `f` is applied to the underlying guard, transforming it into a new guard
    /// with the mapped data. The lock remains in write mode until the guard is explicitly upgraded.
    pub fn map_guard<F, H>(self, f: F) -> SafeGuard<L, H>
    where
        F: FnOnce(G) -> H,
    {
        SafeGuard {
            lock: self.lock,
            guard: f(self.guard),
        }
    }

    /// Attempts to map the guarded value to a different type, returning an error if the mapping fails.
    pub fn try_map_guard<F, H>(self, f: F) -> Result<SafeGuard<L, H>, Self>
    where
        F: FnOnce(G) -> Result<H, G>,
    {
        match f(self.guard) {
            Ok(guard) => Ok(SafeGuard {
                lock: self.lock,
                guard,
            }),
            Err(guard) => Err(SafeGuard {
                lock: self.lock,
                guard,
            }),
        }
    }

    /// Attempts to map the guarded value to a different type, returning an error with additional information if it fails.
    pub fn try_map_guard_err<F, H, E>(self, f: F) -> Result<SafeGuard<L, H>, (Self, E)>
    where
        F: FnOnce(G) -> Result<H, (G, E)>,
    {
        match f(self.guard) {
            Ok(guard) => Ok(SafeGuard {
                lock: self.lock,
                guard,
            }),
            Err((guard, err)) => Err((
                SafeGuard {
                    lock: self.lock,
                    guard,
                },
                err,
            )),
        }
    }
}

impl<L, G, T> Deref for SafeGuard<L, G>
where
    G: Deref<Target = T>,
{
    type Target = T;

    /// Provides read-only access to the underlying value.
    ///
    /// Mutation is only possible after calling [`upgrade`](Self::upgrade).
    fn deref(&self) -> &Self::Target {
        Deref::deref(&self.guard)
    }
}
