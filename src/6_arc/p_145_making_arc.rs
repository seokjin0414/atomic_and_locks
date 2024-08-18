use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::{fence, AtomicUsize};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct ArcMake<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for ArcMake<T> {}
unsafe impl<T: Send + Sync> Sync for ArcMake<T> {}

impl<T> ArcMake<T> {
    pub fn new(data: T) -> ArcMake<T> {
        ArcMake {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) ->&ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> Deref for ArcMake<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

impl<T> Clone for ArcMake<T> {
    fn clone(&self) -> Self {

        if self.data().ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }

        ArcMake {
            ptr: self.ptr
        }
    }
}

impl<T> Drop for ArcMake<T> {
    fn drop(&mut self) {
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}