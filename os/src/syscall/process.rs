//! Process management syscalls
//!
use alloc::sync::Arc;
use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;
use core::mem::size_of;
// use crate::mm::MapPermission;
use crate::{
    fs::{open_file, OpenFlags},
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::{translated_refmut, translated_str, VirtAddr, MapPermission},
    task::{
        add_task, current_task, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
    timer::{get_time_ms, get_time_us},
};

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

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
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
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
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
    let current_task = current_task().unwrap();
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task.pid.0
    );
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
    let current_task = current_task().unwrap();
    let current_task_inner = current_task.inner_exclusive_access();
    trace!(
        "kernel:pid[{}] sys_task_info",
        current_task.pid.0
    );
    let current_time = get_time_ms();
    let start_time = current_task_inner.get_start_time();
    let task_info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: (*current_task_inner.get_syscall_times()),
        time: current_time - start_time,
    };
    copy_paddr_vaddr(&task_info, _ti);
    0
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    let current_task = current_task().unwrap();
    let mut current_task_inner = current_task.inner_exclusive_access();   
    trace!(
        "kernel:pid[{}] sys_mmap",
        current_task.pid.0
    );
    if _start % PAGE_SIZE != 0  || _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }
    let memory_set = current_task_inner.get_memory_set();
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

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    let current_task = current_task().unwrap();
    let mut current_task_inner = current_task.inner_exclusive_access();   
    trace!(
        "kernel:pid[{}] sys_munmap",
        current_task.pid.0
    );
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let memory_set = current_task_inner.get_memory_set();
    let start_va = VirtAddr(_start);
    let end_va = VirtAddr(_start + _len);
    unsafe {
        (*memory_set).remove_framed_area(start_va, end_va)
    }
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
    let current_task = current_task().unwrap();
    trace!(
        "kernel:pid[{}] sys_spawn",
        current_task.pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let new_task = current_task.spawn(all_data.as_slice());
        let new_pid = new_task.pid.0;
        // add new task to scheduler
        add_task(new_task);
        
        new_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    let current_task = current_task().unwrap();
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task.pid.0
    );
    if _prio < 2 {
        -1
    }
    else {
        current_task.inner_exclusive_access().set_priority(_prio as usize);
        _prio
    }
}
