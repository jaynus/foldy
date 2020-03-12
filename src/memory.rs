use crate::{Access, DirEntry};
use fxhash::FxHashMap;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
    sync::Arc,
};

enum FileLock<'a> {
    Read(RwLockReadGuard<'a, MemoryFileData>),
    ReadWrite(RwLockWriteGuard<'a, MemoryFileData>),
}

pub struct MemoryFile<'a> {
    data: FileLock<'a>,
}

struct MemoryFileData {}

enum MemoryFileDataEntry {
    Empty,
    File(RwLock<MemoryFileData>),
}

pub struct MemorySource {
    index: FxHashMap<PathBuf, usize>,
    files: Vec<MemoryFileDataEntry>,
}
impl MemorySource {
    fn read_dir<P>(path: P) -> MemoryDirIter
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        unimplemented!()
    }

    fn open<P>(&self, path: P, access: Access) -> std::io::Result<MemoryFile<'_>>
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        let path = path.as_ref();
        if let Some(data) = self.index.get(path) {
            match self
                .files
                .get(*data)
                .ok_or(Error::new(ErrorKind::NotFound, path.display().to_string()))?
            {
                MemoryFileDataEntry::File(data) => Ok(MemoryFile {
                    data: match access {
                        Access::Read => FileLock::Read(data.read()),
                        Access::ReadWrite => FileLock::ReadWrite(data.write()),
                    },
                }),
                MemoryFileDataEntry::Empty => {
                    Err(Error::new(ErrorKind::NotFound, path.display().to_string()))
                }
            }
        } else {
            Err(Error::new(ErrorKind::NotFound, path.display().to_string()))
        }
    }
}

pub struct MemoryDirIter;
impl Iterator for MemoryDirIter {
    type Item = std::io::Result<DirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn create_vfs() {}
}
