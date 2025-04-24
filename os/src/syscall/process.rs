//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};

// ****** START xisanlou add at ch3 0402 No.1
// 第五章注释掉
//use crate::task::get_a_syscall_times;
//use core::arch::asm;
use core::slice::from_raw_parts;
//use core::slice::from_raw_parts_mut;
// ****** END xisanlou add at ch3 0402 No.1

// ****** START xisanlou add at ch4 0407 No.1
// 第五章注释掉
//const VA_WIDTH_SV39: usize = 39;
use crate::task::{/*current_task_vpn_readable, current_task_vpn_writeable,*/ 
    current_user_insert_framed_area, current_user_vpn_no_overlap, current_user_unmap_user_area,};
use crate::mm::{translated_byte_buffer, VirtAddr, MapPermission};
use crate::timer::get_time_us;
use core::{mem::size_of};
// ****** END xisanlou add at ch4 0407 No.1

// ****** START xisanlou add at ch5 0421 No.1
use crate::config::BIG_STRIDE;
use crate::task::current_user_set_pass;
// ****** END   xisanlou add at ch5 0421 No.1

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );

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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap",
        current_task().unwrap().pid.0
    );

    // ****** START xisanlou add at ch4 0407 No.2
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

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap ",
        current_task().unwrap().pid.0
    );
    // ****** START xisanlou add at ch4 0407 No.3
    
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
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn ",
        current_task().unwrap().pid.0
    );
    // ****** START xisanlou add at ch5 0421 No.2
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let new_pid = new_task.pid.0;
        // modify trap context of new_task, because it returns immediately after switching
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        // we do not have to move to next instruction since we have done it before
        // for child process, fork returns 0
        trap_cx.x[10] = 0;
        // add new task to scheduler
        add_task(new_task);
        return new_pid as isize;
        
    } else {
        return -1;
    }
    // ****** END   xisanlou add at ch5 0421 No.2
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    // ****** START xisanlou add at ch5 0421 No.3
    // Priority mut mo be smaller than 2.
    if _prio < 2 {
        return -1;
    }

    // Compute pass for stride
    let pass: u64 = BIG_STRIDE / (_prio as u64);
    // Set pass in current user TCB
    current_user_set_pass(pass);
    _prio
    // ****** END   xisanlou add at ch5 0421 No.3
}
