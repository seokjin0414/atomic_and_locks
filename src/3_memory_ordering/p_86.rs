use std::{
    sync::atomic::{AtomicUsize, AtomicU64, AtomicBool},
    thread,
    time::Duration
};
use std::cmp::max;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use std::time::Instant;

static DATA: AtomicU64 = AtomicU64::new(0);
static READY: AtomicBool = AtomicBool::new(false);

fn main() {
    thread::spawn(|| {
        DATA.store(123, Relaxed);
        READY.store(true, Release);
    });
    while !READY.load(Acquire) {
        thread::sleep(Duration::from_millis(100));
        println!("waiting.....");
    }
    println!("{}", DATA.load(Relaxed));
}