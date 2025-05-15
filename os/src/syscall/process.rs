//! Process management syscalls
use crate::mm::VirtAddr;
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next,
                    get_user_pa,get_syscall_cnt,mmap,munmap};
use crate::timer::get_time_us;

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
    let us = get_time_us();
    let sec = us / 1_000_000;
    let usec = us % 1_000_000;
    
    fn write_user(addr: VirtAddr, value: usize) -> Result<(), ()> {
        let (mut readable, mut writable) = (false, false);
        if let Some(pa) = get_user_pa(addr, &mut readable, &mut writable) {
            if writable {
                unsafe {
                    *(pa.0 as *mut usize) = value;
                }
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    let ts_addr = VirtAddr(_ts as usize);
    let ts_next = unsafe { VirtAddr((_ts as *mut usize).add(1) as usize) };

    if write_user(ts_addr, sec).is_err() || write_user(ts_next, usec).is_err() {
        -1
    } else {
        0
    }
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    let (mut readable, mut writable) = (false, false);
    let id = VirtAddr(_id);
    match _trace_request {
        0 => get_user_pa(id, &mut readable, &mut writable)
            .map(|pa| {
                if readable {
                    unsafe { *(pa.0 as *const u8) as isize }
                } else {
                    -1
                }
            })
            .unwrap_or(-1),

        1 => get_user_pa(id, &mut readable, &mut writable)
            .map(|pa| {
                if writable {
                    unsafe {
                        *(pa.0 as *mut u8) = (_data & 0xFF) as u8;
                    }
                    0
                } else {
                    -1
                }
            })
            .unwrap_or(-1),

        2 => {let cnt = get_syscall_cnt()[_id] as isize;
              cnt
            },

        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _prot: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    if let Some(_) = mmap(_start, _len, _prot) {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    if let Some(_) = munmap(_start, _len) {
        0
    } else {
        -1
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
