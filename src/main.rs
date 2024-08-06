use std::{
    sync::atomic::{AtomicUsize, AtomicU64},
    thread,
    time::Duration
};
use std::cmp::max;
use std::sync::atomic::Ordering::Relaxed;
use std::time::Instant;

fn main() {

}

fn get_key() -> u64 {
    static KEY: AtomicU64 = AtomicU64::new(0);
    let key = KEY.load(Relaxed);

    if key == 0 {
        let new_key = generate_random_key();
        match KEY.compare_exchange(0, new_key, Relaxed, Relaxed) {
            Ok(_) => new_key,
            Err(k) => k,
        }
    } else {
        key
    }
}