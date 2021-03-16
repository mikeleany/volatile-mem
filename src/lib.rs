//! This crate provides a type, [`Volatile`], for managing volatile memory or
//! data.
//!
//! The [`Volatile`] object does not contain a pointer or reference to the
//! volatile memory, but is a container of the volatile data itself. This means
//! that a pointer to a [`Volatile`] object is a pointer to the volatile memory.
//! As such, it would make little or no sense to create a local variable or
//! parameter of type [`Volatile`]. You would typically use some kind of pointer
//! or reference to the [`Volatile`] object instead.
//!
//! Besides [`Volatile`], the crate provides two additional volatile types. They
//! are [`VolatileReadOnly`], and [`VolatileWriteOnly`]. These are technically
//! just type definitions which alias read-only and write-only variants of
//! [`Volatile`], respectively.  However, those variants are only available
//! through these aliases. The default variant for [`Volatile`] allows both
//! reads and writes.
//!
//! [`Volatile`] is meant for reading from or writing to memory used for
//! communication with some process external to the program. A common use case
//! would be memory-mapped I/O.
//!
//! # Safety
//! Typically, [`Volatile`] would be created from a raw pointer, which carries
//! with it the typical [pointer safety concerns](core::ptr#safety). In
//! particular, the following must be guaranteed regarding the location of the
//! volatile memory.
//!
//! - The memory must be [valid](core::ptr#safety) for reads and/or writes.
//!
//! - The memory must be properly aligned.
//!
//! - The memory must point to a properly initialized for the data type, unless
//! the [`Volatile`] is [write-only](VolatileWriteOnly).
//!
//! Note that even if the data has size zero, the pointer must be non-NULL and
//! properly aligned.
//!
//! Do not forget that even creating a reference to uninitialized data (even if
//! that data is never used) is immediate undefined behavior. As such, do not at
//! any point create a reference directly to uninitialized data (as opposed to a
//! reference to [`VolatileWriteOnly`] or [`MaybeUninit`](core::mem::MaybeUninit),
//! each of which can safely handle uninitialized data).
//!
//! Just like in C, whether an operation is volatile has no bearing whatsoever
//! on questions involving concurrent access from multiple threads. Volatile
//! accesses behave exactly like non-atomic accesses in that regard. In
//! particular, a race between a write operation and any other operation
//! (reading or writing) to the same location is undefined behavior.
//!
//! # Disclaimer
//! The Rust documentation contains the following note regarding volatile reads
//! and writes:
//!
//! > Rust does not currently have a rigorously and formally defined memory
//! > model, so the precise semantics of what "volatile" means here is subject
//! > to change over time. That being said, the semantics will almost always end
//! > up pretty similar to [C11's definition of volatile][c11].
//! >
//! > The compiler shouldn't change the relative order or number of volatile
//! > memory operations. However, volatile memory operations on zero-sized types
//! > [...] are noops and may be ignored.
//!
//! [c11]: http://www.open-std.org/jtc1/sc22/wg14/www/docs/n1570.pdf
#![no_std]
#![allow(unused_unsafe)] // stable alternative to #![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![warn(clippy::todo)]
#![warn(clippy::unimplemented)]
#![warn(clippy::unwrap_used)]
#![deny(safe_packed_borrows)]

mod volatile;
pub use volatile::{Volatile, VolatileReadOnly, VolatileWriteOnly};

/// A marker trait for volatile types.
///
/// This trait must be implemented in order to implement [`VolatileRead`] and
/// [`VolatileWrite`], which will read or write data of type `T`.
pub trait VolatileData<T> {}

/// Volatile data which can be read.
///
/// The data to be read is of type `T`.
pub trait VolatileRead<T>
where
    Self: VolatileData<T>,
    T: Copy,
{
    /// Performs a volatile read of the value in `self` without moving it. This
    /// leaves the memory in `self` unchanged.
    ///
    /// # Safety
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a read operation any write
    /// operation to the same location is undefined behavior.
    fn read(&self) -> T;
}

/// Volatile data which can be written.
///
/// The data to be written is of type `T`.
pub trait VolatileWrite<T>
where
    Self: VolatileData<T>,
    T: Copy,
{
    /// Performs a volatile write of `self` with the given value without reading
    /// the old value.
    ///
    /// # Safety
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a write operation any other
    /// operation (reading or writing) to the same location is undefined
    /// behavior.
    fn write(&mut self, val: T);
}

/// Data which is, or can be treated as, a readable slice of volatile elements.
///
/// The data to be read is of type [`[U]`](slice).
///
/// This trait has a blanket implementation for all types which meet the
/// criteria.
pub trait VolatileReadSlice<T, U>
where
    Self: AsRef<[T]>,
    T: VolatileRead<U>,
    U: Copy,
{
    /// Performs a volatile read of each element of `self` copying the data to
    /// `dst`. This leaves the memory in `self` unchanged.
    ///
    /// The length of `dst` must be the same as `self`.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    ///
    /// # Safety
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a read operation any write
    /// operation to the same location is undefined behavior.
    fn read_slice_volatile(&self, dst: &mut [U]) {
        let this = self.as_ref();
        assert!(
            this.len() == dst.len(),
            "source slice length ({}) does not match destination slice length ({})",
            this.len(),
            dst.len()
        );

        for i in 0..this.len() {
            dst[i] = this[i].read();
        }
    }
}

impl<S, T, U> VolatileReadSlice<T, U> for S
where
    S: AsRef<[T]>,
    T: VolatileRead<U>,
    U: Copy,
{
}

/// Data which is, or can be treated as, a writable slice of volatile elements.
///
/// The data to be written is of type [`[U]`](slice).
///
/// This trait has a blanket implementation for all types which meet the
/// criteria.
pub trait VolatileWriteSlice<T, U>
where
    Self: AsMut<[T]>,
    T: VolatileWrite<U>,
    U: Copy,
{
    /// Performs a volatile write of each element of the slice with the given
    /// value without reading the old data from `self`.
    ///
    /// # Safety
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a write operation any other
    /// operation (reading or writing) to the same location is undefined
    /// behavior.
    fn fill_volatile(&mut self, val: U) {
        let this = self.as_mut();
        for elem in this.iter_mut() {
            elem.write(val);
        }
    }

    /// Performs a volatile write of each element of `self`, copying the data
    /// from `src`, without reading the old data from `self`.
    ///
    /// The length of `src` must be the same as `self`.
    ///
    /// # Panics
    ///
    /// This function will panic if the two slices have different lengths.
    ///
    /// # Safety
    /// Just like in C, whether an operation is volatile has no bearing
    /// whatsoever on questions involving concurrent access from multiple
    /// threads. Volatile accesses behave exactly like non-atomic accesses in
    /// that regard. In particular, a race between a write operation any other
    /// operation (reading or writing) to the same location is undefined
    /// behavior.
    fn write_slice_volatile(&mut self, src: &[U]) {
        let this = self.as_mut();
        assert!(
            this.len() == src.len(),
            "source slice length ({}) does not match destination slice length ({})",
            src.len(),
            this.len()
        );

        for i in 0..this.len() {
            this[i].write(src[i]);
        }
    }
}

impl<S, T, U> VolatileWriteSlice<T, U> for S
where
    S: AsMut<[T]>,
    T: VolatileWrite<U>,
    U: Copy,
{
}
