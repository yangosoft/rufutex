use libc::c_void;
//use log::debug;
use std::sync::atomic::{AtomicU32, Ordering::SeqCst};

/// Mutex implementation based on https://eli.thegreenplace.net/2018/basics-of-futexes/ of the
/// Ulrich Drepper's Futexes are Tricky paper https://www.akkadia.org/drepper/futex.pdf
/// UNLOCKED 0 means unlocked
/// LOCKED_NO_WAITERS 1 means locked, no waiters
/// LOCKED_WAITERS 2 means locked, there are waiters in lock()
use crate::{LOCKED_NO_WAITERS, LOCKED_WAITERS, UNLOCKED};

pub struct SharedFutex {
    pub futex: *mut c_void,
    atom: *mut AtomicU32,
}

impl SharedFutex {
    /// Create a new SharedFutex
    /// # Arguments
    /// * `futex` - A mutable pointer to a c_void
    /// # Returns
    /// A new SharedFutex
    pub fn new(futex: *mut c_void) -> Self {
        let atom: *mut AtomicU32 = futex as *mut AtomicU32;
        Self { futex, atom }
    }

    /// Compare and exchange atomically
    /// This is a wrapper around the compare_exchange method of AtomicU32
    /// It returns the value of the atomic variable before the operation
    /// If the value is different from expected, the operation is not performed
    /// and the value of the atomic variable is returned
    /// If the value is equal to expected, the value of the atomic variable is set to desired
    /// and the value of the atomic variable before the operation is returned
    /// # Arguments
    /// * `atom` - A mutable reference to an AtomicU32
    /// * `expected` - The value to compare with the value of the atomic variable
    /// * `desired` - The value to set the atomic variable to if the value of the atomic variable is equal to expected
    /// # Returns
    /// The value of the atomic variable before the operation
    fn cmpxchg(atom: *mut AtomicU32, expected: u32, desired: u32) -> u32 {
        unsafe {
            match (*atom).compare_exchange(expected, desired, SeqCst, SeqCst) {
                Err(err) => err,
                Ok(val) => val,
            }
        }
    }

    /// Syscall futex
    /// # Arguments
    /// * `futex_op` - The futex operation
    /// * `value` - The value to pass to the futex operation
    /// * `val3` - The third value to pass to the futex operation
    /// # Returns
    /// The result of the syscall
    pub unsafe fn syscall_futex(&mut self, futex_op: i32, value: u32, val3: u32) -> i64 {
        libc::syscall(libc::SYS_futex, self.futex, futex_op, value, 0, 0, val3)
    }

    /// Syscall futex
    /// # Arguments
    /// * `futex_op` - The futex operation
    /// * `value` - The value to pass to the futex operation
    /// * `val2` - The second value to pass to the futex operation
    /// * `val3` - The third value to pass to the futex operation
    /// # Returns
    /// The result of the syscall
    pub unsafe fn syscall_futex3(
        &mut self,
        futex_op: i32,
        value: u32,
        val2: u32,
        val3: u32,
    ) -> i64 {
        libc::syscall(libc::SYS_futex, self.futex, futex_op, value, 0, val2, val3)
    }

    /// Post a futex
    /// # Arguments
    /// * `number_of_waiters` - The number of waiters to notify
    /// # Returns
    /// the ret value of the syscall
    /// Nothing
    pub fn post(&mut self, number_of_waiters: u32) -> i64 {
        unsafe {
            let s = self.syscall_futex(libc::FUTEX_WAKE, number_of_waiters, 0);
            s
        }
    }

    /// Post a futex
    /// # Arguments
    /// * `number_of_waiters` - The number of waiters to notify
    /// * `value` - The value to set the futex to
    /// # Returns
    /// the ret value of the syscall
    /// Nothing
    pub fn post_with_value(&mut self, value: u32, number_of_waiters: u32) -> i64 {
        unsafe {
            (*self.atom).store(value, SeqCst);
            let s = self.syscall_futex(libc::FUTEX_WAKE, number_of_waiters, 0);
            s
        }
    }

    /// Sets the value of the futex
    /// # Arguments
    /// * `value` - The value to set the futex to
    /// # Returns
    /// Nothing
    pub fn set_futex_value(&mut self, value: u32) {
        unsafe {
            (*self.atom).store(value, SeqCst);
        }
    }

    /// Wait on a futex
    /// # Arguments
    /// * `wait_value` - The value to wait on
    /// # Returns
    /// the ret value of the syscall
    pub fn wait(&mut self, wait_value: u32) -> i64 {
        unsafe {
            let ret = self.syscall_futex(libc::FUTEX_WAIT, wait_value, 0);

            ret
        }
    }

    /// Wait on a futex
    /// # Arguments
    /// * `wait_value` - The value to wait on
    /// # Returns
    /// the ret value of the syscall
    pub fn wait_with_timeout(&mut self, wait_value: u32, timeout: *mut libc::timespec) -> i64 {
        unsafe {
            let ptr_timeout: u32 = timeout as u32;
            let ret = self.syscall_futex3(libc::FUTEX_WAIT, wait_value, ptr_timeout, 0);

            ret
        }
    }

    /// Lock the futex
    pub fn lock(&mut self) {
        let mut ret = Self::cmpxchg(self.atom, UNLOCKED, LOCKED_NO_WAITERS);

        // If the lock was previously unlocked, there's nothing else for us to do.
        // Otherwise, we'll probably have to wait.
        if ret != 0 {
            loop {
                // If the mutex is locked, we signal that we're waiting by setting the
                // atom to 2. A shortcut checks is it's LOCKED_WAITERS already and avoids the atomic
                // operation in this case.
                if (ret == LOCKED_WAITERS)
                    || (Self::cmpxchg(self.atom, LOCKED_NO_WAITERS, LOCKED_WAITERS) != UNLOCKED)
                {
                    // Here we have to actually sleep, because the mutex is actually
                    // locked. Note that it's not necessary to loop around this syscall;
                    // a spurious wakeup will do no harm since we only exit the do...while
                    // loop when atom_ is indeed 0.
                    //self.syscall_futex(libc::FUTEX_WAIT, 2, 0);
                    self.wait(LOCKED_WAITERS);
                }
                // We're here when either:
                // (a) the mutex was in fact unlocked (by an intervening thread).
                // (b) we slept waiting for the atom and were awoken.
                //
                // So we try to lock the atom again. We set teh state to 2 because we
                // can't be certain there's no other thread at this exact point. So we
                // prefer to err on the safe side.
                ret = Self::cmpxchg(self.atom, UNLOCKED, LOCKED_WAITERS);
                if ret == 0 {
                    break;
                }
            }
        }
    }

    /// Unlock the futex
    /// If there are waiters, we wake them up
    /// If there are no waiters, we set the atom to UNLOCKED
    /// # Arguments
    /// * `how_may_waiters` - The number of waiters to wake up
    pub fn unlock(&mut self, how_may_waiters: u32) {
        //let val = self.atom;
        let ret: u32;
        unsafe {
            ret = (*self.atom).fetch_sub(1, SeqCst);
        }

        if ret != LOCKED_NO_WAITERS {
            unsafe {
                (*self.atom).store(UNLOCKED, SeqCst);
                self.post(how_may_waiters);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    //use std::intrinsics::atomic_cxchg_acqrel_acquire;

    use super::*;
    use rushm::posixaccessor::POSIXShm;
    use std::mem;
    use std::sync::atomic;
    use std::sync::atomic::AtomicU32;
    use std::sync::mpsc;
    use std::{thread, time};
    #[test]
    fn test_atomic_in_shared_memory() {
        let mut shm = POSIXShm::<i32>::new("futex".to_string(), mem::size_of::<u32>());
        unsafe {
            let ret = shm.open();
            assert!(ret.is_ok());
            ret.unwrap();
        }
        let ptr = shm.get_cptr_mut();

        let a1: *mut AtomicU32 = ptr as *mut AtomicU32;
        unsafe {
            (*a1).store(7, atomic::Ordering::SeqCst);
            let ret = (*a1).load(atomic::Ordering::SeqCst);
            assert_eq!(ret, 7);
        }

        unsafe {
            let ret = shm.close(true);
            assert!(ret.is_ok());
            ret.unwrap();
        }
    }

    #[test]
    fn test_cmpxchg() {
        let mut atomic_val: AtomicU32 = AtomicU32::new(UNLOCKED);
        let before = atomic_val.load(atomic::Ordering::SeqCst);
        let ret = SharedFutex::cmpxchg(&mut atomic_val, UNLOCKED, LOCKED_NO_WAITERS);
        assert_eq!(before, UNLOCKED);
        assert_eq!(ret, before);
    }

    #[test]
    fn test_cmpxchg_shm() {
        unsafe {
            let mut shm =
                POSIXShm::<i32>::new("test_cmpxchg_shm".to_string(), std::mem::size_of::<u32>());

            let ret = shm.open();
            assert!(ret.is_ok());

            let ptr = shm.get_cptr_mut();

            let atom_val: *mut AtomicU32 = ptr as *mut AtomicU32;
            (*atom_val).store(0xFF, atomic::Ordering::SeqCst);

            let before = (*atom_val).load(atomic::Ordering::SeqCst);
            let ret = SharedFutex::cmpxchg(atom_val, UNLOCKED, LOCKED_NO_WAITERS);
            assert_eq!(before, 0xFF);
            assert_eq!(ret, before);

            let ret = shm.close(true);
            assert!(ret.is_ok());
        }
    }

    /*#[test]
    fn test_futex_in_shared_memory() {
        let (tx, rx) = mpsc::channel();
        let mut shm = POSIXShm::<i32>::new(
            "test_futex_in_shared_memory".to_string(),
            std::mem::size_of::<u32>(),
        );
        unsafe {
            let ret = shm.open();
            assert!(ret.is_ok());
        }
        let ptr_shm = shm.get_cptr_mut();

        let shared_atom_val: *mut AtomicU32 = ptr_shm as *mut AtomicU32;

        unsafe {
            (*shared_atom_val).store(LOCKED_NO_WAITERS, atomic::Ordering::SeqCst);
            let val = (*shared_atom_val).load(atomic::Ordering::SeqCst);
            assert_eq!(val, LOCKED_NO_WAITERS);
        }

        let mut sh1 = SharedFutex::new(ptr_shm);

        let handle = thread::spawn(move || {
            let mut shm = POSIXShm::<i32>::new(
                "test_futex_in_shared_memory".to_string(),
                std::mem::size_of::<u32>(),
            );
            unsafe {
                let ret = shm.open();
                assert!(ret.is_ok());
            }
            let ptr_shm = shm.get_cptr_mut();

            let mut sh1 = SharedFutex::new(ptr_shm);
            tx.send(true).unwrap();
            sh1.wait(LOCKED_NO_WAITERS);
        });

        let _ = rx.recv().unwrap();

        sh1.post(1);

        handle.join().unwrap();
        unsafe {
            let ret = shm.close(true);
            assert!(ret.is_ok());
        }
    }*/

    #[test]
    fn test_futex_lock_in_shared_memory() {
        let (tx, rx) = mpsc::channel();
        let mut shm = POSIXShm::<i32>::new(
            "test_futex_lock_in_shared_memory".to_string(),
            std::mem::size_of::<u32>(),
        );
        unsafe {
            let ret = shm.open();
            assert!(ret.is_ok());
        }

        let ptr_shm = shm.get_cptr_mut();
        let shared_atom_val: *mut AtomicU32 = ptr_shm as *mut AtomicU32;

        unsafe {
            (*shared_atom_val).store(LOCKED_NO_WAITERS, atomic::Ordering::SeqCst);
            let val = (*shared_atom_val).load(atomic::Ordering::SeqCst);
            assert_eq!(val, LOCKED_NO_WAITERS);
        }

        let mut shared_futex = SharedFutex::new(ptr_shm);

        let handle = thread::spawn(move || {
            let mut shm = POSIXShm::<i32>::new(
                "test_futex_lock_in_shared_memory".to_string(),
                std::mem::size_of::<u32>(),
            );
            unsafe {
                let ret = shm.open();
                assert!(ret.is_ok());
            }
            let ptr_shm = shm.get_cptr_mut();
            let mut shared_futex = SharedFutex::new(ptr_shm);
            tx.send(true).unwrap();
            shared_futex.lock();
        });

        let _ = rx.recv().unwrap();

        // wait a few ms to make sure the other thread is in the lock function
        thread::sleep(time::Duration::from_millis(500));
        shared_futex.unlock(1);

        handle.join().unwrap();
        unsafe {
            let ret = shm.close(true);
            assert!(ret.is_ok());
        }
    }

    #[test]
    fn test_shared_lock_unlock() {
        let mut shm = POSIXShm::<i32>::new("test_shared_lock_unlock".to_string(), 8);
        unsafe {
            let ret = shm.open();
            assert!(ret.is_ok());
        }
        let ptr_shm = shm.get_cptr_mut();
        let mut shared_futex = SharedFutex::new(ptr_shm);

        shared_futex.lock();
        shared_futex.unlock(1);
        shared_futex.lock();
        shared_futex.unlock(1);

        // Cleanup
        unsafe {
            let ret = shm.close(true);
            assert!(ret.is_ok());
        }
    }
}
