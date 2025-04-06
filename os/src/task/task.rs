//! Types related to task management

use super::TaskContext;
// ****** START xisanlou add at ch3 0402 No.1
use crate::config::MAX_SYSCALL_NUM;
// ****** END xisanlou add at ch3 0402 No.1

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    // ****** START xisanlou add at ch3 0402 No.2
    /// The task syscall times
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    // ****** END xisanlou add at ch3 0402 No.2
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
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
