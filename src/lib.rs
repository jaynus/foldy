#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), alloc)]

#[cfg(feature = "std")]
pub use std;

#[cfg(not(feature = "std"))]
pub mod std {
    extern crate alloc as alloc;
    extern crate core as _internal_std;
    pub use _internal_std::*;
}

pub mod path {
    #[cfg(feature = "std")]
    pub use crate::std::path::{Iter, Path, PathBuf};

    #[cfg(not(feature = "std"))]
    pub struct Path {}

    #[cfg(not(feature = "std"))]
    pub struct PathBuf {}

    #[cfg(not(feature = "std"))]
    pub struct Iter {}
}
use path::*;

#[derive(thiserror::Error, PartialEq, Debug)]
pub enum FoldyError {
    #[error("lol")]
    FileNotFound,
    #[error("lol")]
    DirectoryNotFound,
    #[error("lol")]
    InvalidPath,
    #[error("lol")]
    EOF,
}

#[cfg(feature = "std")]
impl Into<std::io::Error> for FoldyError {
    fn into(self) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", self))
    }
}

pub mod memory;

pub struct DirEntry {
    path: PathBuf,
}
impl DirEntry {
    fn path(&self) -> PathBuf {
        unimplemented!()
    }
}

#[cfg(feature = "std")]
pub trait File: std::io::Write + std::io::Read + std::io::Seek {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FoldyError>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, FoldyError>;
    fn seek(&mut self, pos: u64) -> Result<u64, FoldyError>;
    fn position(&self) -> u64;
}

#[cfg(not(feature = "std"))]
pub trait File {}

pub trait Source<'a> {
    type DirIter: 'a + Iterator<Item = Result<DirEntry, FoldyError>>;

    fn read_dir<P>(&'a self, path: P) -> Result<Self::DirIter, FoldyError>
    where
        P: 'a + AsRef<Path>,
        Self: Sized;

    fn create_dir<P>(&mut self, path: P) -> Result<(), FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized;

    fn remove_dir<P>(&mut self, path: P) -> Result<(), FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized;

    fn open<P>(&self, path: P) -> Result<&dyn File, FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized;

    fn open_mut<P>(&mut self, path: P) -> Result<&mut dyn File, FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized;
}
