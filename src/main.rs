use std::cell::UnsafeCell;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::sync::atomic::{fence, AtomicU8};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::Thread;
use std::{
    sync::atomic::{AtomicBool, AtomicU64, AtomicUsize},
    thread,
    time::Duration,
};

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
        let mut n = self.data().data_ref_count.load(Relaxed);

        loop {
            if n == 0 {
                return None;
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
        unsafe { (*ptr).as_ref().unwrap() }
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

fn main() {
    use itertools::Itertools;
    use std::collections::BTreeMap;
    use std::time::Instant;

    #[derive(Clone)]
    struct UserSolutionHistoryQ {
        userId: uuid::Uuid,
        data: String,
    }

    let data_sizes = [10, 50, 100, 1_000, 5_000, 10_000, 100_000, 1_000_000]; // 데이터 크기

    for &size in &data_sizes {
        println!("\n데이터 크기: {}", size);

        // 데이터 생성
        let user_history_q: Vec<UserSolutionHistoryQ> = (0..size)
            .map(|i| UserSolutionHistoryQ {
                userId: uuid::Uuid::new_v4(),
                data: format!("data_{}", i),
            })
            .collect();

        // Fold 방식
        let start = Instant::now();
        let _map_fold: BTreeMap<uuid::Uuid, Vec<UserSolutionHistoryQ>> =
            user_history_q.iter().fold(BTreeMap::new(), |mut map, q| {
                map.entry(q.userId).or_default().push(q.clone());
                map
            });
        println!("Fold 시간: {:?}", start.elapsed());

        // itertools 방식
        let start = Instant::now();
        let _map_itertools: HashMap<uuid::Uuid, Vec<UserSolutionHistoryQ>> = user_history_q
            .iter()
            .cloned()
            .into_group_map_by(|q| q.userId);
        println!("itertools 시간: {:?}", start.elapsed());
    }
}

#[test]
fn test() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);
    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    // 2개 weak pointer 에서 Arc 생성
    let x = ArcMake::new(("hi", DetectDrop));
    let y = ArcMake::downgrade(&x);
    let z = ArcMake::downgrade(&x);

    let t = std::thread::spawn(move || {
        // upgrade weak pointer
        let y = y.upgrade().unwrap();
        assert_eq!(y.0, "hi");
    });
    assert_eq!(x.0, "hi");
    t.join().unwrap();

    // data 는 아직 메모리에서 삭제 안됨
    // weak pointer upgrade 가능
    assert_eq!(NUM_DROPS.load(Relaxed), 0);
    assert!(z.upgrade().is_some());

    drop(x);

    // deleted data
    // weak pointer upgrade 불가능
    assert_eq!(NUM_DROPS.load(Relaxed), 1);
    assert!(z.upgrade().is_none());
}
