use rufutex::rufutex::SharedFutex;
use rushm::posixaccessor::POSIXShm;
use std::thread;

fn wait_locked() {
    let mut shm = POSIXShm::<i32>::new("test_shared_lock_unlock".to_string(), 8);
    unsafe {
        let ret = shm.open();
        assert!(ret.is_ok());
    }
    let ptr_shm = shm.get_cptr_mut();
    let mut shared_futex = SharedFutex::new(ptr_shm);
    println!("Thread id {:?} waiting for lock", thread::current().id());
    shared_futex.lock();
    println!("Thread id {:?} got the lock", thread::current().id());
    shared_futex.unlock(1);
}
fn main() {
    // Your code here
    let mut shm = POSIXShm::<i32>::new("test_shared_lock_unlock".to_string(), 8);
    unsafe {
        let ret = shm.open();
        assert!(ret.is_ok());
    }
    let ptr_shm = shm.get_cptr_mut();
    let mut shared_futex = SharedFutex::new(ptr_shm);

    shared_futex.lock();

    let handles: Vec<_> = (0..2)
        .map(|_| {
            thread::spawn(|| {
                wait_locked();
            })
        })
        .collect();

    println!("Main Thread waiting to spawn the threads");
    // Wait some time to spawn the threads
    thread::sleep(std::time::Duration::from_secs(5));
    println!("Main Thread id {:?} unlocking", thread::current().id());
    shared_futex.unlock(1);

    

    shared_futex.lock();
    shared_futex.unlock(1);

    // Cleanup
    for handle in handles {
        handle.join().unwrap();
    }
    unsafe {
        let ret = shm.close(true);
        assert!(ret.is_ok());
    }
}
