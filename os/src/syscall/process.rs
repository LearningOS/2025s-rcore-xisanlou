//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};
// ****** START xisanlou add at ch3 0402 No.1
use crate::task::get_a_syscall_times;
use core::arch::asm;
use core::slice::from_raw_parts;
use core::slice::from_raw_parts_mut;
// ****** END xisanlou add at ch3 0402 No.1

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // ****** START xisanlou add at ch3 0402 No.2
    
    let result: isize;
    result = match _trace_request {
        0 => unsafe{(_id as *const u8).read_volatile() as isize},
        1 => {
            unsafe{
                let id_src = from_raw_parts(
                    _id as *const u8, 1
                );
                let id_dst = from_raw_parts_mut(
                    _data as *mut u8, 1
                );
                id_dst.copy_from_slice(id_src);
            }
            0
        },
        2 => get_a_syscall_times(_id) as isize,
        _ => -1,
    };

    unsafe {
        asm!("fence.i");
    }

    result
    // ****** END xisanlou add at ch3 0402 No.2
}
