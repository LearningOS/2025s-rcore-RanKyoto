# 第五章练习
## 一、简单总结你实现的功能
本章使用了操作系统中经典的 fork-exec-wait 模型。它的作用是：
> 父进程 fork 出子进程 → 子进程调用 exec 运行用户程序 → 父进程等待子进程退出 → 打印退出信息

在了解了上述功能之后，我们实现了`spwan`, 创建一个新的独立进程，直接加载指定程序，该进程绑定在当前进程的子进程上。 同样父进程负责监视子进程是否结束，子进程来执行某个 elf 文件。 此外我们也实现了 stride 算法。

## 二、简答作业：stride 算法深入
> stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， p1.stride = 255, p2.stride = 250，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。实际情况是轮到 p1 执行吗？为什么？

- 当 p2 执行一个时间片之后：p2.pass += 250 --> p2.pass = 10 + 250 = 260，但由于是 8-bit 无符号整数 --> 溢出成 260 % 256 = 4
```
p1.pass = 10
p2.pass = 4
```
比较这两个数，结果是 p2 < p1，调度器会错误地再次选择 p2，造成 p1 饿死。

> 为什么进程优先级 priority ≥ 2 可以限制 STRIDE 差距 ≤ BigStride / 2？

令：
- 所有 stride 取值为 BigStride / priority
- 若最小 priority = 2 ⇒ 最大 stride = BigStride / 2
- 若最大 priority = ∞ ⇒ 最小 stride → 0

所以两个进程之间最大的 stride 差异为：
> Stride_max - Stride_min ≤ BigStride / 2 - ε

然后，每轮增加 pass 的差值也最多为 stride 差值。所以若任务 share ≥ 2，任意两个 pass 的差值最多为 BigStride / 2，不会越过溢出边界。

---

```rust
const BIG_STRIDE: u64 = 256;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let diff = self.0.wrapping_sub(other.0) % BIG_STRIDE;
        if diff == 0 {
            None // 它们不相等，逻辑上不应该走到这里，因为 PartialEq::eq 返回 false
        } else if diff < BIG_STRIDE / 2 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, _other: &Self) -> bool {
        false // 根据题意：两个 stride 永远不相等
    }
}

```