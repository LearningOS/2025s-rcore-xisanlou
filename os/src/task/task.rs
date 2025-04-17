//! Types related to task management
use super::TaskContext;
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE,
};
use crate::trap::{trap_handler, TrapContext};


// ****** START xisanlou add at ch3 0402 No.1
use crate::config::MAX_SYSCALL_NUM;
// ****** END xisanlou add at ch3 0402 No.1

// ****** START xisanlou add at ch4 0407 No.1
use crate::mm::VirtPageNum;
// ****** END xisanlou add at ch4 0407 No.1

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,

    /// Application address space
    pub memory_set: MemorySet,

    /// The phys page number of trap context
    pub trap_cx_ppn: PhysPageNum,

    /// The size(top addr) of program which is loaded from elf file
    pub base_size: usize,

    /// Heap bottom
    pub heap_bottom: usize,

    /// Program break
    pub program_brk: usize,

    // ****** START xisanlou add at ch3 0402 No.2
    /// The task syscall times
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    // ****** END xisanlou add at ch3 0402 No.2
}

impl TaskControlBlock {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// get the user token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// Based on the elf info in program, build the contents of task in a new address space
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            // ****** START xisanlou add at ch3 0402 No.2 ch4 update
            syscall_times: [0; MAX_SYSCALL_NUM],
            // ****** END xisanlou add at ch3 0402 No.2
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }

    // ****** START xisanlou add at ch4 0407 No.2
    /// 检查虚拟页的可读性
    pub fn vpn_readable(&self, vpn: VirtPageNum) -> bool {
        if let Some(pte) = self.memory_set.translate(vpn) {
            return pte.readable();
        } else {
            return false;
        }
    }

    /// 检查虚拟页的可写性
    pub fn vpn_writeable(&self, vpn: VirtPageNum) -> bool {
        if let Some(pte) = self.memory_set.translate(vpn) {
            return pte.writable();
        } else {
            return false;
        }
    }

    /// insert framed area to user space.
    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        self.memory_set.insert_framed_area(start_va, end_va, permission);
    }

    /// Test VirtAddr range overlapping.
    pub fn vpn_no_overlap(&self, start_va: VirtAddr, end_va: VirtAddr) -> bool {
        self.memory_set.vpn_no_overlap(start_va, end_va)
    }

    /// unmap framed area in user space
    pub fn unmap_user_area(&mut self, start_va: VirtAddr, end_va: VirtAddr) -> isize {
        self.memory_set.unmap_user_area(start_va, end_va)
    }
    // ****** END xisanlou add at ch4 0407 No.2
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
