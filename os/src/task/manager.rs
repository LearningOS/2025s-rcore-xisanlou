//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;

// ****** START xisanlou add at ch5 0421 No.1
//use alloc::collections::VecDeque;
use alloc::collections::binary_heap::BinaryHeap;
use core::cmp::Reverse;
use crate::config::STRIDE_MAX;
// ****** END   xisanlou add at ch5 0421 No.1

use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    // ****** START xisanlou add at ch5 0421 No.2
    //ready_queue: VecDeque<Arc<TaskControlBlock>>,
    ready_queue_0: BinaryHeap<Reverse<Arc<TaskControlBlock>>>,
    ready_queue_1: BinaryHeap<Reverse<Arc<TaskControlBlock>>>,
    queue_0_active: bool,
    rewind_counter: usize,
    rewind_time: bool,
    // ****** END   xisanlou add at ch5 0421 No.2
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            // ****** START xisanlou add at ch5 0421 No.3
            //ready_queue: VecDeque::new(),
            ready_queue_0: BinaryHeap::new(),
            ready_queue_1: BinaryHeap::new(),
            queue_0_active: true,
            rewind_counter: 0,
            rewind_time: false,
            // ****** END   xisanlou add at ch5 0421 No.3
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        // ****** START xisanlou add at ch5 0421 No.4
        //self.ready_queue.push_back(task);

        // 检查stride是否溢出
        let overflow = task.stride_overflow();
        // (1)发生溢出
        if overflow {
            self.rewind_time = true;
            self.rewind_counter += 1;
            // push task 到非活跃队列
            task.stride_add_step();
            if self.queue_0_active {
                self.ready_queue_1.push(Reverse(task));
            } else {
                self.ready_queue_0.push(Reverse(task));
            }
            return;
        }

        // (2)未溢出，但回绕time标记为true
        if self.rewind_time {
            self.rewind_counter += 1;
            // push到active队列
            task.stride_add_step();
            if self.queue_0_active {
                self.ready_queue_0.push(Reverse(task));
            } else {
                self.ready_queue_1.push(Reverse(task));
            }
            return;
        }

        // (3)未溢出，time标记为false，但计数器非0
        if self.rewind_counter > 0 {
            self.rewind_counter -= 1;

            let stride = task.stride_add_step();
            // stride位于取值的后半段，则stride清0
            if stride > STRIDE_MAX / 2 {
                task.stride_clear();
            }
            
            // push到active队列
            if self.queue_0_active {
                self.ready_queue_0.push(Reverse(task));
            } else {
                self.ready_queue_1.push(Reverse(task));
            }
            return;
        }

        // (4)未发生溢出回绕，不需要特殊处理
        task.stride_add_step();
        if self.queue_0_active {
            self.ready_queue_0.push(Reverse(task));
        } else {
            self.ready_queue_1.push(Reverse(task));
        }

        
        // ****** END   xisanlou add at ch5 0421 No.4
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // ****** START xisanlou add at ch5 0421 No.5
        //self.ready_queue.pop_front()

        let  option_r_task: Option<Reverse<Arc<TaskControlBlock>>>;
        let  option_r_task_2: Option<Reverse<Arc<TaskControlBlock>>>;

        // pop from current active queue
        if self.queue_0_active {
            option_r_task = self.ready_queue_0.pop();
        } else {
            option_r_task = self.ready_queue_1.pop();
        }

        match option_r_task {
            // get a task 
            Some(r_task) => {return Some(r_task.0);},
            // active queue is empty
            None => {
                // Switch pop queue
                self.queue_0_active = ! self.queue_0_active;
                // rewind time end
                self.rewind_time = false;
                
                // pop from current active queue
                if self.queue_0_active {
                    option_r_task_2 = self.ready_queue_0.pop();
                } else {
                    option_r_task_2 = self.ready_queue_1.pop();
                }

                match option_r_task_2 {
                    Some(r_task) => {return Some(r_task.0);},
                    None => {return None;},
                };
            },
        };

        // ****** END   xisanlou add at ch5 0421 No.5
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
