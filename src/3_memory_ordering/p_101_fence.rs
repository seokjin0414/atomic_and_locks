use std::{
    sync::atomic::{AtomicUsize, AtomicU64, AtomicBool},
    thread,
    time::Duration
};
use std::cmp::max;
use std::sync::atomic::{fence, AtomicPtr};
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};
use std::time::Instant;

static mut DATA: [u64; 10] = [0; 10];

const ATOMIC_FALSE: AtomicBool = AtomicBool::new(false);
static READY: [AtomicBool; 10] = [ATOMIC_FALSE; 10];

fn main() {
    for i in 0..10 {
        thread::spawn(move || {
            let data = some_calculation(i);
            unsafe { DATA[i] = data };
            READY[i].store(true, Release);
        });
    }
    thread::sleep(Duration::from_millis(500));
    let ready: [bool; 10] = std::array::from_fn(|i| READY[i].load(Relaxed));
    if ready.contains(&true) {
        fence(Acquire);
        for i in 0..10 {
            if ready[i] {
                println!("data{i} = {}", unsafe { DATA[i] });
            }
        }
    }
}

fn some_calculation(n: usize) -> u64 {
    thread::sleep(Duration::from_millis(500));
    println!("calculation: {}", n);
    n as u64
}