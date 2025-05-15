.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .align 2
__alltraps:
    csrrw sp, sscratch, sp      //一般为定时器中断进入
    # now sp->*TrapContext in user space, sscratch->user stack
    # save other general purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they have been saved in TrapContext
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp) //将当前任务的寄存器状态传入陷入上下文
    sd t1, 33*8(sp) //记录当前任务的执行位置
    # read user stack from sscratch and save it in TrapContext
    csrr t2, sscratch
    sd t2, 2*8(sp) //记录当前任务的用户栈
    # load kernel_satp into t0
    ld t0, 34*8(sp) //内核地址空间也存在了陷入上下文中
    # load trap_handler into t1
    ld t1, 36*8(sp)
    # move to kernel_sp
    ld sp, 35*8(sp) //陷入到内核态了，所以需要指向任务的内核栈，每个任务都有自己对应的内核栈
    # switch to kernel space
    csrw satp, t0 //t0 是内核地址空间的根页表物理地址的 token
    sfence.vma //修改SATP后需执行sfence.vma指令刷新TLB(Translation Lookaside Buffer)，确保地址转换生效
    # jump to trap_handler
    jr t1 //跳转到 trap_handler

__restore:
    # a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1 //进入到下一个任务的地址空间
    sfence.vma //修改SATP后需执行sfence.vma指令刷新TLB(Translation Lookaside Buffer)，确保地址转换生效
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1     // 当前任务执行所在的位置
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp) //现在 sp 从指向陷入上下文变成指向用户栈
    sret //返回 sepc，继续执行下一个任务
