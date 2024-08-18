use std::{
    sync::atomic::{AtomicUsize, AtomicU64, AtomicBool},
    thread,
    time::Duration
};
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::{fence, AtomicU8};
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::thread::Thread;

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

fn main() {
}














