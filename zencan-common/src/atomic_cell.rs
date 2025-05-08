//! Implements an AtomicCell type which uses critical_section Mutex to enforce atomic store/load
//!
//! Since Crossbeam does not support thumbv6m due to lack of CAS, this is a fallback option.
//!
//! TODO: This can be made more efficient by using core::sync::atomic types for types which can
//! transmute into the base integer types

use critical_section::Mutex;
use core::{cell::Cell, ops::Add};

#[derive(Debug)]
pub struct AtomicCell<T: Copy> {
    inner: Mutex<Cell<T>>,
}

impl<T: Send + Copy> AtomicCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(Cell::new(value)),
        }
    }

    pub fn load(&self) -> T {
        critical_section::with(|cs| {
            self.inner.borrow(cs).get()
        })
    }

    pub fn store(&self, value: T) {
        critical_section::with(|cs| {
            self.inner.borrow(cs).set(value)
        });
    }

    pub fn fetch_update(&self, mut f: impl FnMut(T) -> Option<T>) -> Result<T, T> {
        critical_section::with(|cs| {
            let old_value = self.inner.borrow(cs).get();
            if let Some(new_value) = f(old_value) {
                self.inner.borrow(cs).set(new_value);
                Ok(old_value)
            } else {
                Err(old_value)
            }
        })
    }
}

impl<T: Send + Copy + Default> AtomicCell<T> {
    pub fn take(&self) -> T {
        critical_section::with(|cs| {
            self.inner.borrow(cs).take()
        })
    }
}

impl<T: Copy + Add<Output = T>> AtomicCell<T> {
    pub fn fetch_add(&self, value: T) -> T {
        critical_section::with(|cs| {
            let old_value = self.inner.borrow(cs).get();
            self.inner.borrow(cs).set(old_value + value);
            old_value
        })
    }
}

impl<T: Default + Copy + Send> Default for AtomicCell<T> {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Cell::new(T::default()))
        }
    }
}



