use std::{sync::atomic::AtomicUsize, thread, time::Duration};
use std::sync::atomic::Ordering::Relaxed;

fn main() {
    let num_done = AtomicUsize::new(0);

    thread::scope(|s| {
        s.spawn(|| {
            for i in 0..100 {
                process_item(i);
                num_done.store(i + 1, Relaxed);
            }
        });

        loop {
            let n = num_done.load(Relaxed);
            if n == 100 {
                break;
            }
            println!("working.. {}/100", n);
            thread::sleep(Duration::from_secs(1));
        }
    });
}

fn process_item(i: usize) {
    println!("do!!!");
    thread::sleep(Duration::from_millis(100));
}