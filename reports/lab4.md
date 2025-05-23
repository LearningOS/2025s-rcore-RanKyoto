# 第六和第七章练习
## 一、简单总结你实现的功能
实现了系统调用硬链接和解除硬链接，以及获取DiskInode信息。
其中对 DiskInode新增了链接数字段，初始为 1，表示有多少个目录项对应该文件，当链接数为 0 时，即所有链接解除时，需要对数据块和 DiskInode 进行回收。

## 二、第六章简答作业：在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

root_inode 定义在 `/os/src/fs/inode.rs`
```rust
lazy_static! {
    pub static ref ROOT_INODE: Arc<Inode> = {
        let efs = EasyFileSystem::open(BLOCK_DEVICE.clone());
        Arc::new(EasyFileSystem::root_inode(&efs))
    };
}
```
它代表虚拟 MMIO -- BLOCK_DEVICE 的根目录，存放着目录项（文件名和文件对应的 inode_id），根目录损坏会导致我们不能正确地找到文件和对应的数据块。

## 二、第七章简答作业：举出使用 pipe 的一个实际应用的例子。
在 Linux shell 中，可以用下面这条命令统计一个文件的行数：

> cat file.txt | wc -l

这背后的实现，其实就是用一个 pipe（管道） 把 cat 命令的输出传给 wc -l 的输入。

- cat file.txt 产生的输出（stdout）写入 pipe。
- wc -l 从 pipe 的读取端（stdin）读取数据并统计行数。