use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    in_use: AtomicBool,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T>
where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            in_use: AtomicBool::new(false),
            ready: AtomicBool::new(false),
        }
    }

    // message 2개 이상인 경우 panic
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, Relaxed) {
            panic!("cant send more than one message!");
        }
        unsafe { (*self.message.get()).write(message) };
        self.ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    // ready() 로 먼저 확인
    pub fn receive(&self) -> T {
        if !self.ready.swap(false, Acquire) {
            panic!("no message available.");
        }
        // 안전함: ready 플래그 확인후 초기화
        unsafe { (*self.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

fn main() {
    let channel = Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hoho");
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }
        assert_eq!(channel.receive(), "hoho");
    });
}