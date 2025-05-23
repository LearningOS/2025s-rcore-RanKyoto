//! virtio_blk device driver

mod virtio_blk;

pub use virtio_blk::VirtIOBlock;

use alloc::sync::Arc;
use easy_fs::BlockDevice;
use lazy_static::*;

type BlockDeviceImpl = virtio_blk::VirtIOBlock;

lazy_static! { //维护下面这个块设备 RefCell 提供单线程内部可变调用， Arc 提供多线程共享
    /// The global block device driver instance: BLOCK_DEVICE with BlockDevice trait
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(BlockDeviceImpl::new());//第一次创建，强计数为 1
}

#[allow(unused)]
/// Test the block device
pub fn block_device_test() {
    let block_device = BLOCK_DEVICE.clone();
    let mut write_buffer = [0u8; 512];
    let mut read_buffer = [0u8; 512];
    for i in 0..512 {
        for byte in write_buffer.iter_mut() {
            *byte = i as u8;
        }
        block_device.write_block(i as usize, &write_buffer);
        block_device.read_block(i as usize, &mut read_buffer);
        assert_eq!(write_buffer, read_buffer);
    }
    println!("block device test passed!");
}
