//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE}, mm::VirtAddr, task::{
        change_program_brk, exit_current_and_run_next, get_currect_syscall_times, get_current_memory_set, get_current_start_time, suspend_current_and_run_next, TaskStatus
    }, timer::{get_time_ms, get_time_us}
};

use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;
use core::mem::size_of;
use crate::mm::MapPermission;
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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

/// copy data from src(physical address) to dst(virtual address)
fn copy_paddr_vaddr<T> (src: &T, dst: *mut T) {
    let src_array = unsafe { 
        core::slice::from_raw_parts(src as *const _ as *const u8, size_of::<T>()) 
    };
    let buffers = translated_byte_buffer(current_user_token(), dst as *const u8, size_of::<T>());
    let mut start = 0;
    for buffer in buffers {
        if size_of::<T>() - start > buffer.len() {
            buffer.copy_from_slice(&src_array[start..start + buffer.len()]);
        }
        else {
            buffer.copy_from_slice(&src_array[start..]);
        }
        start += buffer.len();
    } 
}
/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    copy_paddr_vaddr(&time_val, _ts);
    0
}
/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let current_time = get_time_ms();
    let start_time = get_current_start_time();
    let task_info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: get_currect_syscall_times(),
        time: current_time - start_time,
    };
    copy_paddr_vaddr(&task_info, _ti);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if _start % PAGE_SIZE != 0  || _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }
    let memory_set = get_current_memory_set();
    let start_va = VirtAddr(_start);
    let end_va = VirtAddr(_start + _len);
    println!("start_va: {:?}, end_va: {:?}", start_va, end_va);
    unsafe {
        if (*memory_set).check_mapped_status(start_va, end_va) != 0 { // mapped
            return -1;
        }
        // please use union instead of & (& is no defined for MapPermission)
        (*memory_set).insert_framed_area(start_va, 
                            end_va, 
                            MapPermission::from_bits((_port << 1) as u8).unwrap().union(MapPermission::U));
        0
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let memory_set = get_current_memory_set();
    let start_va = VirtAddr(_start);
    let end_va = VirtAddr(_start + _len);
    unsafe {
        (*memory_set).remove_framed_area(start_va, end_va)
    }
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
