use std::cell::UnsafeCell;
use std::ptr::NonNull;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;

struct ArcData<T> {
    data_ref_count: AtomicUsize,
    alloc_ref_count: AtomicUsize,
    data: UnsafeCell<Option<T>>,
}

pub struct WeakMake<T> {
    ptr: NonNull<ArcData<T>>,
}

pub struct ArcMake<T> {
    weak: WeakMake<T>,
}

unsafe impl<T: Send + Sync> Send for WeakMake<T> {}
unsafe impl<T: Send + Sync> Sync for WeakMake<T> {}

impl<T> ArcMake<T> {
    pub fn new(data: T) -> ArcMake<T> {
        ArcMake {
            weak: WeakMake {
                ptr: NonNull::from(Box::leak(Box::new(ArcData {
                    data_ref_count: AtomicUsize::new(1),
                    alloc_ref_count: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.weak.data().alloc_ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            // 안전함: Arc가 단 한개 존제, Weak 는 한개도 없어
            // 현재 Arc가 독점적 접근 가능
            let arcdata = unsafe { arc.weak.ptr.as_mut() };
            let option = arcdata.data.get_mut();
            // data 를 가리키는 Arc가 있어 panic X
            let data = option.as_mut().unwrap();
            Some(data)
        } else {
            None
        }
    }

    pub fn downgrade(arc: &Self) -> WeakMake<T> {
        arc.weak.clone()
    }
}

impl<T> WeakMake<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn upgrade(&self) -> Option<ArcMake<T>> {
        let mut n = self.data().data_ref_count.load(Relaxed );

        loop {
            if n == 0 {
                return  None;
            }
            assert!(n < usize::MAX);
            if let Err(e) =
                self.data()
                    .data_ref_count
                    .compare_exchange_weak(n, n + 1, Relaxed, Relaxed)
            {
                n = e;
                continue;
            }
            return Some(ArcMake { weak: self.clone() });
        }
    }
}

impl<T> Deref for ArcMake<T> {
    type Target = T;

    fn deref(&self) -> &T {
        let ptr = self.weak.data().data.get();
        // 안전함 Arc가 data를 가리키고 있어
        // data는 존재하고 공유될수 있다
        unsafe {(*ptr).as_ref().unwrap() }
    }
}

impl<T> Clone for WeakMake<T> {
    fn clone(&self) -> Self {
        if self.data().alloc_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        WeakMake { ptr: self.ptr }
    }
}

impl<T> Clone for ArcMake<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();

        if weak.data().data_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        ArcMake { weak }
    }
}

impl<T> Drop for WeakMake<T> {
    fn drop(&mut self) {
        if self.data().alloc_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

impl<T> Drop for ArcMake<T> {
    fn drop(&mut self) {
        if self.weak.data().data_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            let ptr = self.weak.data().data.get();
            // 안정함: data의 레퍼런스 카운터가 0 이므로
            // 이제 data 접근 불가능
            unsafe {
                *ptr = None;
            }
        }
    }
}