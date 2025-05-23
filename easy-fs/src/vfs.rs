use super::{
    block_cache_sync_all, get_block_cache, BlockDevice, DirEntry, DiskInode, DiskInodeType,
    EasyFileSystem, DIRENT_SZ,
};
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, MutexGuard};
/// Virtual filesystem layer over easy-fs
pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// Create a vfs inode
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }
    /// Call a function over a disk inode to read it  
    /// 从缓存中读取设备为 self.block_device, id 为 self.block_id 的块
    /// 在这个块的 self.block_offset 处获取类型为 DiskInode 的不可变引用
    /// 传给泛函 f
    fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .read(self.block_offset, f)
    }
    /// Call a function over a disk inode to modify it
    /// 与上面类似，只不过传回来的是可变引用
    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(self.block_id, Arc::clone(&self.block_device))
            .lock()
            .modify(self.block_offset, f)
    }
    /// Find inode under a disk inode by name
    /// 在目录项中找到名为 name 的 inode的 编号
    fn find_inode_id(&self, name: &str, disk_inode: &DiskInode) -> Option<u32> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device,),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some(dirent.inode_id() as u32);
            }
        }
        None
    }
    /// Find inode under current inode by name
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode).map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    /// 获取 inode 的状态信息
    pub fn stat(&self) -> (i32, u32) {
        let (mode, nlink) = self.read_disk_inode(|disk_inode| {
            let mode = if disk_inode.is_dir() {
                1
            } else {
                2
            };
            let nlink = disk_inode.nlink;
            (mode, nlink as u32)
        });
        (mode, nlink as u32)
    }

    /// 硬链接
    pub fn link(&self, old_name:&str, new_name:&str)-> Option<()>{
        //注意 self.find 锁了文件系统，释放后需要再重新锁一下
        let Some(old_inode) = self.find(old_name) else{
            return None;
        }; //链接对象不存在的时候返回 None
        old_inode.modify_disk_inode(|old_inode: &mut DiskInode| {
            old_inode.nlink += 1;
        });

        let mut fs = self.fs.lock();
        self.modify_disk_inode(|root_inode| {     
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);

            // write dirent
            let old_inode_id= fs.get_disk_inode_id(old_inode.block_id, old_inode.block_offset);
            let dirent = DirEntry::new(new_name, old_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ, //扩容后的起始位置
                dirent.as_bytes(),
                &self.block_device,
            );
            Some(())
        });
        block_cache_sync_all();
        Some(())
    }

    /// 解除硬链接
    pub fn unlink(&self, name:&str)->Option<()>{
        //注意 self.find 锁了文件系统，释放后需要再重新锁一下
        let Some(inode) = self.find(name) else{
            return None;
        }; //链接对象不存在的时候返回 None
        
        let mut fs = self.fs.lock();
        
        // 减少文件的链接计数
        let need_dealloc = inode.modify_disk_inode(|disk_inode: &mut DiskInode| {
            disk_inode.nlink -= 1;
            // 如果链接计数为0，需要回收数据块
            disk_inode.nlink == 0 //返回布尔类型
        });
        
        // 如果链接计数为0，回收数据块和inode
        if need_dealloc {
            // 获取并清除文件的数据块
            inode.modify_disk_inode(|disk_inode: &mut DiskInode| {
                let size = disk_inode.size;
                let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
                assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
                // 回收数据块
                for data_block in data_blocks_dealloc.into_iter() {
                    fs.dealloc_data(data_block);
                }
            });

            // 回收inode
            let inode_id = fs.get_disk_inode_id(inode.block_id, inode.block_offset);
            fs.dealloc_inode(inode_id);
        }
        
        // 从目录中删除文件项
        self.modify_disk_inode(|root_inode| {
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let mut dirent = DirEntry::empty();
            
            // 遍历，找到要删除的文件项的位置
            let mut found_idx = 0;
            for i in 0..file_count {
                assert_eq!(
                    root_inode.read_at(DIRENT_SZ * i, dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );//读取目录项
                if dirent.name() == name {
                    found_idx = i;
                    break;
                }
            }
            
            // 如果是最后一个目录项，直接减小目录大小
            if found_idx == file_count - 1 {
                root_inode.size -= DIRENT_SZ as u32;
            } else {
                // 否则，将最后一个文件项移动到要删除的位置
                let last_idx = file_count - 1;
                let mut last_dirent = DirEntry::empty();
                assert_eq!(
                    root_inode.read_at(DIRENT_SZ * last_idx, last_dirent.as_bytes_mut(), &self.block_device),
                    DIRENT_SZ,
                );
                
                // 将最后一个文件项写入到要删除的位置
                root_inode.write_at(
                    DIRENT_SZ * found_idx,
                    last_dirent.as_bytes(),
                    &self.block_device,
                );
                
                // 减小目录大小
                root_inode.size -= DIRENT_SZ as u32;
            }
        });
        
        block_cache_sync_all();
        Some(())

    }
    /// Increase the size of a disk inode
    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {  //如果新的大小小于原本的大小，则无需扩容
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size); //计算需要新添加的块的数目，包含了新增索引块和数据块
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());// 给新增的块分配地址
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }
    /// Create inode under current inode by name
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        let op = |root_inode: &DiskInode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        };
        if self.read_disk_inode(op).is_some() {
            return None; //如果文件已经存在，则返回 None
        }
        // create a new file
        // alloc a inode with an indirect block
        let new_inode_id = fs.alloc_inode(); //每一个文件都要分配一个 inode
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) = fs.get_disk_inode_pos(new_inode_id);
        get_block_cache(new_inode_block_id as usize, Arc::clone(&self.block_device))
            .lock()
            .modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
                new_inode.initialize(DiskInodeType::File);
            });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ; //第一次创建文件应该为 0
            let new_size = (file_count + 1) * DIRENT_SZ; //每新增一个目录项应该增加一个 DIRENT_SZ
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });//添加目录项完毕

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }
    /// List inodes under current inode
    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(i * DIRENT_SZ, dirent.as_bytes_mut(), &self.block_device,),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }
    /// Read data from current inode
    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| disk_inode.read_at(offset, buf, &self.block_device))
    }
    /// Write data to current inode
    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }
    /// Clear the data in current inode
    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert!(data_blocks_dealloc.len() == DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}
