use std::{
    sync::atomic::{AtomicUsize, AtomicU64, AtomicBool},
    thread,
    time::Duration
};
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::mem::MaybeUninit;
use std::sync::{Condvar, Mutex};
use std::sync::atomic::Ordering::{Acquire, Release};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T>
    where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    // 안전함: is_ready() 가 true 리턴한 다음
    pub unsafe fn receive(&self) -> T {
        (*self.message.get()).assume_init_read()
    }
}

fn main() {
}


