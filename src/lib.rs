use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub mod memory;

pub struct DirEntry {}
impl DirEntry {
    fn path(&self) -> PathBuf {
        unimplemented!()
    }
}

pub enum Access {
    Read,
    ReadWrite,
}
