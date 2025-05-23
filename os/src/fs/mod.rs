//! File trait & inode(dir, file, pipe, stdin, stdout)

mod inode;
mod stdio;

use crate::mm::UserBuffer;

/// trait File for all file types
pub trait File: Send + Sync {
    /// the file readable?
    fn readable(&self) -> bool;
    /// the file writable?
    fn writable(&self) -> bool;
    /// read from the file to buf, return the number of bytes read
    fn read(&self, buf: UserBuffer) -> usize;
    /// write to the file from buf, return the number of bytes written
    fn write(&self, buf: UserBuffer) -> usize;
    /// get the stat of the file
    fn get_stat(&self) -> Option<Stat> {
        None
    }
}

/// The stat of a inode
#[repr(C)]
#[derive(Debug)]
pub struct Stat { //80 字节
    /// ID of device containing file
    pub dev: u64, //8 字节
    /// inode number
    pub ino: u64, //8 字节
    /// file type and mode
    pub mode: StatMode, //4 字节
    /// number of hard links
    pub nlink: u32, //4 字节
    /// unused pad
    pad: [u64; 7], //7*8 字节
}

bitflags! {
    /// The mode of a inode
    /// whether a directory or a file
    pub struct StatMode: u32 {
        /// null
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}

pub use inode::{list_apps, open_file,link_file,unlink_file, OSInode, OpenFlags};
pub use stdio::{Stdin, Stdout};
