# 第八章练习
## 一、简单总结你实现的功能
实现了用银行家算法来检测死锁。该功能可以通过系统调用来开启或者关闭。

## 二、第八章简答作业：

> 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？
- 从 /task/mod.rs 中 99-148行可以得知回收资源的过程，首先，从PID2PCB去除映射，将当前进程标记为僵尸进程，然后将所有的子进程移到初始化进程中，后续处理。然后回收任务列表中的所有任务的TaskUserRes。之后依次清除TaskUserRes，子进程，文件描述符表，和所有线程。最后在初始化进程中处理标记为僵尸进程的子进程。

> 对比以下两种 Mutex 中的实现，二者有什么区别？这些区别可能会导致什么问题？
```rust
impl Mutex for Mutex1 {
     fn lock(&self) {
         loop {
             let mut mutex_inner = self.inner.exclusive_access();
             if mutex_inner.locked {
                 mutex_inner.wait_queue.push_back(current_task().unwrap());
                 drop(mutex_inner);
                 block_current_and_run_next();
             } else {
                mutex_inner.locked = true;
                break;
            }
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        mutex_inner.locked = false;
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        }
    }
}

impl Mutex for Mutex2 {
    fn lock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    fn unlock(&self) {
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            add_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
```
这两种互斥锁对应的 /sync/mutex.rs 中的 `MutexSpin` 和 `MutexBlocking`, 前者在解锁之后，会造成其他任务抢夺锁，不稳定，适合用于使用互斥资源时间短，不频繁的情形。 后者会按照申请调用互斥资源的顺序来调度，比较稳定。