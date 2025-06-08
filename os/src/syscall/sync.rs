use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

//检测死锁，银行家算法
fn detect_deadlock(
    mut available: Vec<isize>,
    alloc: &Vec<Vec<isize>>,
    need: &Vec<Vec<isize>>,
) -> bool {
    let mut finish = vec![false; alloc.len()];//表示每个线程是否能顺利完成任务
    let mut changed = true;//表示每一轮算法中是否至少有一个线程被标记为完成
    while changed {//只要有一轮没完成，就检测为会产生死锁
        changed = false;
        for (index, (task_need, task_alloc)) in need.iter().zip(alloc.iter()).enumerate() {
            if finish[index] {//如果index 线程能完成，就继续检测下一个线程是否满足要求
                continue;
            }//判断当前线程是否满足所有种类的可用资源数目都大于所需求的资源的数目
            if available.iter().zip(task_need).all(|(a, b)| a >= b) {
                available = available
                    .iter()
                    .zip(task_alloc)
                    .map(|(a, b)| a + b)
                    .collect();//如果满足，就认为当前线程能完成，然后回收资源
                finish[index] = true; //标记为完成
                changed = true;
            }
        }
    }
    !finish.iter().all(|&x| x)
}

/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {//在当前进程的互斥量列表中找到一个空的位置，放入新创建的互斥量
        process_inner.mutex_list[id] = mutex;
        process_inner.resource_available[0][id] = 1;
        //分配和所需的互斥量置零，因为刚刚创建
        process_inner.resource_alloc[0]
            .iter_mut()
            .for_each(|resource_alloc| resource_alloc[id] = 0);
        process_inner.resource_need[0]
            .iter_mut()
            .for_each(|resource_need| resource_need[id] = 0);
        id as isize
    } else {//没找到空的位置，在列表最后 push 一个
        process_inner.mutex_list.push(mutex);
        process_inner.resource_available[0].push(1);
        process_inner.resource_alloc[0]
            .iter_mut()
            .for_each(|resource_alloc| resource_alloc.push(0));
        process_inner.resource_need[0]
            .iter_mut()
            .for_each(|resource_need| resource_need.push(0));
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.resource_need[0][tid][mutex_id] += 1; //线程 tid 对 mutex_id 发起请求
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    if detect_deadlock(//检测互斥量是否有死锁
        process_inner.resource_available[0].clone(),
        &process_inner.resource_alloc[0],
        &process_inner.resource_need[0],
    ) {//如果有死锁，退回请求，然后返回-0xDEAD（16 进制）
        process_inner.resource_need[0][tid][mutex_id] -= 1;
        -0xDEAD
    } else { //如果没有死锁
        drop(process_inner);
        drop(process);
        mutex.lock();
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.resource_available[0][mutex_id] -= 1; //锁资源使用
        process_inner.resource_alloc[0][tid][mutex_id] += 1;//锁资源分配
        process_inner.resource_need[0][tid][mutex_id] -= 1; //锁需求解决
        0
    }
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    process_inner.resource_available[0][mutex_id] += 1; //锁资源释放，可用加一
    process_inner.resource_alloc[0][tid][mutex_id] -= 1;
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)//在当前进程的信号量列表中查找是否有空位置
    {// 有空位置就放入信号量，信号量的大小就是可用资源的数目，由于刚创建，分配和需求都为 0
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        process_inner.resource_available[1][id] = res_count as isize;
        process_inner.resource_alloc[1]
            .iter_mut()
            .for_each(|resource_alloc| resource_alloc[id] = 0);
        process_inner.resource_need[1]
            .iter_mut()
            .for_each(|resource_need| resource_need[id] = 0);
        id
    } else {//如果列表没有空位置，就在最后添加一个
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.resource_available[1].push(res_count as isize);
        process_inner.resource_alloc[1]
            .iter_mut()
            .for_each(|resource_alloc| resource_alloc.push(0));
        process_inner.resource_need[1]
            .iter_mut()
            .for_each(|resource_need| resource_need.push(0));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    let tid = current_task()
    .unwrap()
    .inner_exclusive_access()
    .res
    .as_ref()
    .unwrap()
    .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.resource_alloc[1][tid][sem_id] -= 1; //释放信号量资源
    process_inner.resource_available[1][sem_id] += 1;  //可用+1
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.resource_need[1][tid][sem_id] += 1; //线程 tid 请求信号量 sim_id
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    if process_inner.detect_deadlock
    && detect_deadlock(//检测信号量请求是否会产生死锁
        process_inner.resource_available[1].clone(),
        &process_inner.resource_alloc[1],
        &process_inner.resource_need[1],
    )
{ //如果会，就退回请求
    process_inner.resource_need[1][tid][sem_id] -= 1;
    -0xDEAD
} else { //如果不会产生死锁
    drop(process_inner);
    sem.down();
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    process_inner.resource_available[1][sem_id] -= 1; //信号量资源使用了 1 个
    process_inner.resource_alloc[1][tid][sem_id] += 1;//信号量资源分配出去了 1 个到 线程 tid
    process_inner.resource_need[1][tid][sem_id] -= 1; //需求量-1
    0
}
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    match enabled {
        0 => {
            process_inner.detect_deadlock = false;
            0
        }
        1 => {
            process_inner.detect_deadlock = true;
            0
        }
        _ => -1,
    }
}
