# 第三章练习
## 一、简单总结你实现的功能
在`TaskControlBlock`中增加了一个`pub syscall_times: [u32;MAX_SYSCALL_NUM]`用于统计各个任务调用了多少次用户态`syscall`，其中不同种类的调用分别按照 ID 来统计。

在`task/mod.rs`中初始化计数器，并且增加了两个私有的函数，分别负责统计和调用。在实例外部增加两个公共函数用来调用全局变量`TASK_MANAGER`的值。

最后在`syscall/mod.rs`中调用统计，在`syscall/process.rs`中实现练习中要求的内容。

## 二、简答作业
- ### 问题 1
> 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (ch2b_bad_*.rs) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

出错行为分别如下：  
[ch2b_bad_address.rs] 试图写`0x0000_0000`地址
```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
```
由于 rustsbi-qemu 的内存配置如下:
```
[rustsbi] pmp01: 0x00000000..0x80000000 (-wr)
[rustsbi] pmp02: 0x80000000..0x80200000 (---)
[rustsbi] pmp03: 0x80200000..0x88000000 (xwr)
[rustsbi] pmp04: 0x88000000..0x00000000 (-wr)
```
其中用户态(U)指令`write_volatile`只能访问权限为(xwr)的地址。可以发现`(0x87FF_FFFF as *mut u8).write_volatile(0);`可以通过测试，但是`(0x8800_0000 as *mut u8).write_volatile(0);` 不能通过。但是随便写这部分的内存应该可能会造成系统出错，因为应用程序和 rustsbi-qemu.bin 也在这个区域内。

[ch2b_bad_instructions.rs]试图使用监督态(S)指令`sret`，然而当前为用户态(U)。同样[ch2b_bad_register.rs]中，`core::arch::asm!("csrr {}, sstatus", out(reg) sstatus);`尝试调用监督态(S)的寄存器`sstatus`，所以也是监督态(S)指令。因此这两个应用程序的执行结果如下：
```
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.   
```
SBI 的版本如下：
```
[rustsbi] RustSBI version 0.4.0, adapting to RISC-V SBI v2.0.0
[rustsbi] RustSBI-QEMU Version 0.2.0-alpha.3
```
---
- ### 问题 2
>深入理解 trap.S 中两个函数 __alltraps 和 __restore 的作用，并回答如下问题:
>1. L40：刚进入 __restore 时，sp 代表了什么值。请指出 __restore 的两种使用情景。

sp是内核栈指针，指向内核栈的栈顶。 全局搜索`__restore`，发现只有TaskContext.ra 存储了`__restore`的地址。这代表 switch 完成之后，会调用ra地址处的函数，即恢复下一个任务的TrapContext。

对于第一个任务，用一个空的任务跟第一个任务做 switch, `__restore`起到启动第一个任务的作用。

>2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。
```
    ld t0, 32*8(sp)   // __restore 恢复的是下一个(第一个)任务的 TRAP_CONTEXT
    ld t1, 33*8(sp)   // sp 指向下一个(第一个)任务的内核栈的栈顶
    ld t2, 2*8(sp)
    csrw sstatus, t0  // 恢复下一个(第一个)任务的处理器的状态
    csrw sepc, t1     // 恢复下一个任务(第一个)需要执行的下一条指令地址
    csrw sscratch, t2 // 恢复下一个任务(第一个)的内核栈的栈顶指针
```

> 3. L50-L56：为何跳过了 x2 和 x4？

- x2 就是 内核栈指针 sp 无需再配置 
- x4 在程序执行过程中不发生改变，无需配置。

> 4. L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```
    csrrw sp, sscratch, sp
```
- sp 指针将指向下一个任务(第一个)的用户栈的栈顶 
- sscratch 保存的是下一个任务(第一个)的内核栈的栈顶

> 5. __restore：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

在 `sret` 指令。因为会从监督态(S)返回到原来的状态，继续执行 spec 指向地址的程序，在这里就是用户态。

> 6. L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？
```
    csrrw sp, sscratch, sp
```

- sp 指针将指向当前任务的内核栈的栈顶 
- sscratch 保存的是当前任务的用户栈的栈顶

> 7. 从 U 态进入 S 态是哪一条指令发生的？

`ecall`指令来实现，在这段代码中没有直接体现，在 `trap_handler` 中的 `sbi_call` 的调用中可以体现。

## 三、荣誉准则
1. 在完成本次实验的过程（含此前学习的过程）中，我参考了许善朴的2022春季OS课实验框架讲解，还在代码中对应的位置以注释形式记录了具体的参考来源及内容


3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。