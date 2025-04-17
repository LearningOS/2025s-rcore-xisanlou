//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
// ****** START xisanlou add at ch3 0402 No.1
use crate::task::get_a_syscall_times;
//use core::arch::asm;
use core::slice::from_raw_parts;
use core::slice::from_raw_parts_mut;
// ****** END xisanlou add at ch3 0402 No.1

// ****** START xisanlou add at ch4 0407 No.1
const VA_WIDTH_SV39: usize = 39;
use crate::task::{current_task_vpn_readable, current_task_vpn_writeable, current_user_token,
    current_user_insert_framed_area, current_user_vpn_no_overlap, current_user_unmap_user_area,};
use crate::mm::{translated_byte_buffer, VirtAddr, MapPermission};
use crate::timer::get_time_us;
use core::{mem::size_of};
// ****** END xisanlou add at ch4 0407 No.1

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // ****** START xisanlou add at ch4 0407 No.2 from-2024102601
    let buffers = translated_byte_buffer(
        current_user_token(),
        _ts as *const u8,
        size_of::<TimeVal>(),
    );
    if buffers.len() == 0 {
        return -1;
    }

    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    let mut offset = 0;
    for buffer in buffers {
        unsafe {
            let src = from_raw_parts((&time_val as *const TimeVal as *const u8).wrapping_add(offset), buffer.len());
            buffer.copy_from_slice(src);
        }
        offset += buffer.len();
    }
    
    0
    // ****** END xisanlou add at ch4 0407 No.2 from-2024102601
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // ****** START xisanlou add at ch3 0402 No.2 ch40407update
    
    match _trace_request {
        0 => {
            // 检查传入_id是否在SV39有效地址空间
            if _id > (1<<VA_WIDTH_SV39) -1 && _id < !((1<<VA_WIDTH_SV39) -1) {return -1;}
            // 检查地址可读性
            let va: VirtAddr = _id.into();
            if current_task_vpn_readable(va.floor()) == false {return -1;}
            
            // 从用户空间复制数据到内核空间
            let buffers = translated_byte_buffer(
                current_user_token(),
                _id as *const u8,
                size_of::<u8>(),
            );
            if buffers.len() == 0 {
                return -1;
            }

            let mut result: isize = 0;

            let mut offset = 0;
            for buffer in buffers {
                unsafe {
                    let dst = from_raw_parts_mut((&mut result as *mut isize as *mut u8).wrapping_add(offset), buffer.len());
                    dst.copy_from_slice(buffer);
                }
                offset += buffer.len();
            }
            return result;
        },
        1 => {
            // 检查传入_id是否在SV39有效地址空间
            if _id > (1<<VA_WIDTH_SV39) -1 && _id < !((1<<VA_WIDTH_SV39) -1) {return -1;}
            // 检查地址可写性
            let va: VirtAddr = _id.into();
            if current_task_vpn_writeable(va.floor()) == false {return -1;}

            // 复制_data到用户地址空间
            let buffers = translated_byte_buffer(
                current_user_token(),
                _id as *const u8,
                size_of::<u8>(),
            );
            if buffers.len() == 0 {
                return -1;
            }

            let mut offset = 0;
            for buffer in buffers {
                unsafe {
                    let src = from_raw_parts((&_data as *const usize as *const u8).wrapping_add(offset), buffer.len());
                    buffer.copy_from_slice(src);
                }
                offset += buffer.len();
            }
            return 0;
        },
        2 => return get_a_syscall_times(_id) as isize,
        _ => return -1,
    };

    // ****** END xisanlou add at ch3 0402 No.2 ch40407update
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
   // ****** START xisanlou add at ch4 0407 No.2
   trace!("kernel: sys_mmap");
    // get start and end Virtual addrsss.
    let start_va = VirtAddr::from(_start);
    if ! start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(_start + _len);

    // Test port and change it to MapPermission.
    if ((_port & !0x7) != 0) || ((_port & 0x7) == 0) {
        return -1;
    }

    let mut map_perm = MapPermission::from_bits((_port as u8) << 1).unwrap();
    map_perm.set(MapPermission::U, true);
    
    // Test virtual address range overlapping.
    if ! current_user_vpn_no_overlap(start_va, end_va) {
        return -1;
    }

    // map virtual address to physical address.
    current_user_insert_framed_area(start_va, end_va, map_perm);

    0
   // ****** END xisanlou add at ch4 0407 No.2
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    // ****** START xisanlou add at ch4 0407 No.3
    trace!("kernel: sys_munmap ");
    // get start and end Virtual addrsss.
    let start_va = VirtAddr::from(_start);
    if ! start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(_start + _len);
    if ! end_va.aligned() {
        return -1;
    }

    current_user_unmap_user_area(start_va, end_va)
    // ****** END xisanlou add at ch4 0407 No.3
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
