//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
use crate::config::{MAX_STRIDE, BIG_STRIDE};
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        let mut mi = MAX_STRIDE;
        let mut index = 0;
        for i in 0..self.ready_queue.len() {
            let stride = self.ready_queue[i].inner_exclusive_access().stride;
            let is_overflow = self.ready_queue[i].inner_exclusive_access().is_overflow;
            if !is_overflow && stride < mi {
                mi = self.ready_queue[i].inner_exclusive_access().stride;
                index = i;
            }
        }
        if self.ready_queue.len() > 0 && mi == MAX_STRIDE { // all overflow -> all not overflow
            for i in 0..self.ready_queue.len() {
                let stride = self.ready_queue[i].inner_exclusive_access().stride;
                self.ready_queue[i].inner_exclusive_access().is_overflow = false;
                if stride < mi {
                    mi = self.ready_queue[i].inner_exclusive_access().stride;
                    index = i;
                }
            }
        }
        if mi == MAX_STRIDE { // len == 0
            None
        } else {
            let priority = self.ready_queue[index].inner_exclusive_access().get_priority();
            let pass = BIG_STRIDE / priority as u64;
            self.ready_queue[index].inner_exclusive_access().stride += pass;
            if self.ready_queue[index].inner_exclusive_access().stride >= MAX_STRIDE { // avoid overflow
                self.ready_queue[index].inner_exclusive_access().is_overflow = true;
                self.ready_queue[index].inner_exclusive_access().stride -= MAX_STRIDE;
            }
            Some(self.ready_queue.remove(index))
        }
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
