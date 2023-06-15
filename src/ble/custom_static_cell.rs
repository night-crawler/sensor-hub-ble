use core::cell::UnsafeCell;
use core::mem::MaybeUninit;

use atomic_polyfill::{AtomicBool, Ordering};

pub struct CustomStaticCell<T> {
    used: AtomicBool,
    val: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for CustomStaticCell<T> {}

unsafe impl<T> Sync for CustomStaticCell<T> {}

impl<T> CustomStaticCell<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            used: AtomicBool::new(false),
            val: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init(&'static self, val: T) -> &'static mut T {
        self.uninit().write(val)
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init_ro(&'static self, val: T) -> &'static T {
        self.uninit().write(val)
    }

    pub fn get(&'static self) -> &'static T {
        unsafe {
            (*self.val.get()).assume_init_ref()
        }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn init_with(&'static self, val: impl FnOnce() -> T) -> &'static mut T {
        self.uninit().write(val())
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub fn uninit(&'static self) -> &'static mut MaybeUninit<T> {
        if self
            .used
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            panic!("StaticCell::init() called multiple times");
        }

        unsafe { &mut *self.val.get() }
    }
}
