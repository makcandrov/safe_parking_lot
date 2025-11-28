# safe_parking_lot

A lightweight wrapper around `parking_lot::RwLock` that leverages Rust’s
borrow checker to guarantee correct conditional write patterns.

When writing code that may or may not modify the contents of an `RwLock`
depending on runtime conditions, it is easy to accidentally:

- mutate before the condition is validated,
- mutate multiple times inside a retry loop,
- leak guards across iterations,
- or continue a loop after modifying the data.

All of these can lead to subtle logic bugs that the compiler cannot detect
when using a raw `RwLock`.

`SafeRwLock` prevents these patterns entirely by using the Rust type system.

## Example

In this example we repeatedly lock a value until a condition is satisfied,
and then write exactly once:

```rust
use parking_lot::RwLock;
use safe_parking_lot::SafeRwLock;

let lock = RwLock::new(10usize);
let mut safe = SafeRwLock::new(&lock);

loop {
    // Acquire a temporary guard that allows inspection but not mutation.
    let guard = safe.lock();

    // We only want to modify the value once it reaches 20.
    if *guard < 20 {
        // Condition not met: unlock and continue retrying.
        safe = guard.unlock();
        continue;
    }

    // Condition met: explicitly convert to a write guard.
    let mut writable = guard.upgrade();
    *writable = 0;

    break; // mutation occurs exactly once
}
```

## Example of incorrect usage (will not compile)

The following code attempts to modify the data and then continue the loop,
allowing multiple writes. The compiler rejects this when using `SafeRwLock`:

```rust,compile_fail
use parking_lot::RwLock;
use safe_parking_lot::SafeRwLock;

let lock = RwLock::new(10usize);
let mut safe = SafeRwLock::new(&lock);

loop {
    let guard = safe.lock();

    if *guard < 20 {
        safe = guard.unlock();
        continue;
    }

    // Attempting to upgrade and write...
    let mut writable = guard.upgrade();
    *writable = 123;

    // ...but also continuing the loop afterwards.
    // With a raw `RwLock` this silently compiles.
    // With SafeRwLock this fails to compile, ensuring correctness.
}
```
