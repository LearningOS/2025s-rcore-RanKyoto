//! Types related to task management

use super::TaskContext;
/// We can find the Biggest num is SYSCALL_CONDVAR_WAIT: usize = 473, 
/// so, let us set it to 500
pub const MAX_SYSCALL_NUM:usize = 500;
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in its lifecycle
    pub task_status: TaskStatus,
    /// Count how many times the syscall funciton is called
    pub syscall_times: [u32;MAX_SYSCALL_NUM], /// the index is the ID of syscall
    /// The task context
    pub task_cx: TaskContext,
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
