use crate::{DirEntry, File, FoldyError, Source};

#[cfg(not(feature = "std"))]
use crate::std;

use crate::FoldyError::DirectoryNotFound;
use crate::{
    path::{Iter as PathIter, Path, PathBuf},
    std::collections::{HashMap, HashSet},
};

#[derive(Debug, Clone)]
pub enum MemoryEntry {
    File(MemoryFile),
    Directory(HashMap<PathBuf, MemoryEntry>),
}
impl MemoryEntry {
    pub fn visit_mut(&mut self, mut path: &Path) -> Result<&mut MemoryEntry, FoldyError> {
        if path.starts_with("/") {
            path = path
                .strip_prefix("/")
                .map_err(|_| FoldyError::InvalidPath)?;
        }

        let mut iter = path.iter();
        if let Some(part) = iter.next() {
            let path = Path::new(part);
            match self {
                MemoryEntry::Directory(map) => map
                    .get_mut(path)
                    .ok_or(FoldyError::DirectoryNotFound)?
                    .visit_mut(iter.as_path()),
                _ => Err(FoldyError::InvalidPath),
            }
        } else {
            Ok(self)
        }
    }

    pub fn visit(&self, mut path: &Path) -> Result<&MemoryEntry, FoldyError> {
        if path.starts_with("/") {
            path = path
                .strip_prefix("/")
                .map_err(|_| FoldyError::InvalidPath)?;
        }

        let mut iter = path.iter();
        if let Some(part) = iter.next() {
            let path = Path::new(part);
            match self {
                MemoryEntry::Directory(map) => map
                    .get(path)
                    .ok_or(FoldyError::DirectoryNotFound)?
                    .visit(iter.as_path()),
                _ => Err(FoldyError::InvalidPath),
            }
        } else {
            Ok(self)
        }
    }
}

#[derive(Default, Clone)]
pub struct MemoryFile {
    pub data: Vec<u8>,
    pub stream_offset: usize,
}
impl std::fmt::Debug for MemoryFile {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "File ( size = {} )", self.data.len())
    }
}
impl MemoryFile {
    const GROWTH_RATE: usize = 2;

    // desired, actual
    pub fn grow(&mut self, desired_capacity: u64) -> Result<u64, FoldyError> {
        let new_size = (desired_capacity as usize).max(self.data.len() * Self::GROWTH_RATE);
        self.data.resize(new_size, 0);
        Ok(new_size as u64)
    }

    pub fn from_slice(buf: &[u8]) -> Self {
        Self {
            stream_offset: 0,
            data: buf.to_vec(),
        }
    }
}
impl File for MemoryFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, FoldyError> {
        let end = (self.stream_offset + buf.len()).min(self.data.len());
        let len = end - self.stream_offset;

        if len > 0 {
            buf[0..len].copy_from_slice(&self.data[self.stream_offset..end]);
            self.stream_offset = end;
            Ok(len)
        } else {
            Ok(0)
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, FoldyError> {
        if self.stream_offset + buf.len() > self.data.len() {
            self.grow((self.stream_offset + buf.len()) as u64)?;
        }

        let end = (self.stream_offset + buf.len()).min(self.data.len());
        let len = end - self.stream_offset;
        self.data[self.stream_offset..end].copy_from_slice(buf);

        self.stream_offset = end;
        Ok(len)
    }

    fn position(&self) -> u64 {
        self.stream_offset as u64
    }

    fn seek(&mut self, pos: u64) -> Result<u64, FoldyError> {
        let new_offset = pos.min(self.data.len() as u64) as usize;
        if new_offset > self.data.len() {
            self.grow(new_offset as u64)?;
        }
        self.stream_offset = new_offset;

        Ok(self.stream_offset as u64)
    }
}
#[cfg(feature = "std")]
impl std::io::Read for MemoryFile {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        File::read(self, buf).map_err(FoldyError::into)
    }
}
#[cfg(feature = "std")]
impl std::io::Write for MemoryFile {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        File::write(self, buf).map_err(FoldyError::into)
    }

    fn write_all(&mut self, mut buf: &[u8]) -> std::io::Result<()> {
        unimplemented!()
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[cfg(feature = "std")]
impl std::io::Seek for MemoryFile {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        use std::io::SeekFrom;
        match pos {
            SeekFrom::Start(offset) => {
                if self.data.len() < offset as usize {
                    self.grow(offset);
                }
                self.stream_offset = offset as usize;
                Ok(offset)
            }
            SeekFrom::End(offset) => {
                let new_offset = self.data.len() as i64 + offset;
                if new_offset < 0 {
                    self.stream_offset = 0;
                    return Ok(0);
                }
                let new_offset = new_offset as usize;
                if self.data.len() < new_offset {
                    self.grow(new_offset as u64);
                }
                self.stream_offset = new_offset;
                Ok(new_offset as u64)
            }
            SeekFrom::Current(offset) => {
                let new_offset = (self.stream_offset as i64) + offset;
                if new_offset < 0 {
                    self.stream_offset = 0;
                    return Ok(0);
                }
                let new_offset = new_offset as usize;
                if self.data.len() < new_offset {
                    self.grow(new_offset as u64);
                }
                self.stream_offset = new_offset;
                Ok(new_offset as u64)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MemorySource {
    root: MemoryEntry,
}
impl Default for MemorySource {
    fn default() -> Self {
        Self {
            root: MemoryEntry::Directory(HashMap::default()),
        }
    }
}
impl MemorySource {}
impl<'a> Source<'a> for MemorySource {
    type DirIter = MemoryDirIter<'a, std::collections::hash_map::Iter<'a, PathBuf, MemoryEntry>>;

    fn read_dir<P>(&'a self, path: P) -> Result<Self::DirIter, FoldyError>
    where
        P: 'a + AsRef<Path>,
        Self: Sized,
    {
        match self
            .root
            .visit(path.as_ref().parent().unwrap_or(Path::new("")))?
        {
            MemoryEntry::File(_) => Err(FoldyError::InvalidPath),
            MemoryEntry::Directory(ref map) => Ok(MemoryDirIter {
                iter: map.iter(),
                root: path.as_ref().to_path_buf(),
                _marker: std::marker::PhantomData::default(),
            }),
        }
    }

    fn create_dir<P>(&mut self, path: P) -> Result<(), FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        let path = path.as_ref();
        let parent = self.root.visit_mut(path.parent().unwrap())?;
        let filename = &path.file_name().ok_or(FoldyError::InvalidPath)?;
        let end_path = Path::new(filename);

        match parent {
            MemoryEntry::File(_) => Err(FoldyError::InvalidPath),
            MemoryEntry::Directory(ref mut map) => {
                map.entry(end_path.to_path_buf())
                    .or_insert_with(|| MemoryEntry::Directory(HashMap::default()));
                Ok(())
            }
        }
    }

    fn remove_dir<P>(&mut self, path: P) -> Result<(), FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        unimplemented!()
    }

    fn open<P>(&self, path: P) -> Result<&dyn File, FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        let entry = self.root.visit(path.as_ref())?;
        match entry {
            MemoryEntry::File(ref file) => Ok(file),
            _ => Err(FoldyError::InvalidPath),
        }
    }

    fn open_mut<P>(&mut self, path: P) -> Result<&mut dyn File, FoldyError>
    where
        P: AsRef<Path>,
        Self: Sized,
    {
        let path = path.as_ref();
        let parent = self.root.visit_mut(path.parent().unwrap())?;
        let filename = &path.file_name().ok_or(FoldyError::InvalidPath)?;
        let end_path = Path::new(filename);

        match parent {
            MemoryEntry::File(_) => Err(FoldyError::InvalidPath),
            MemoryEntry::Directory(ref mut map) => Ok(
                match map
                    .entry(end_path.to_path_buf())
                    .or_insert_with(|| MemoryEntry::File(MemoryFile::default()))
                {
                    MemoryEntry::File(ref mut file) => file,
                    MemoryEntry::Directory(_) => unreachable!(),
                },
            ),
        }
    }
}

pub struct MemoryDirIter<'a, I> {
    root: PathBuf,
    iter: I,
    _marker: std::marker::PhantomData<&'a ()>,
}
impl<'a, I> Iterator for MemoryDirIter<'a, I>
where
    I: Iterator<Item = (&'a PathBuf, &'a MemoryEntry)> + ExactSizeIterator,
{
    type Item = Result<DirEntry, FoldyError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            Some(Ok(DirEntry {
                path: self.root.join(next.0).to_path_buf(),
            }))
        } else {
            None
        }
    }
}
impl<'a, I> ExactSizeIterator for MemoryDirIter<'a, I>
where
    I: Iterator<Item = (&'a PathBuf, &'a MemoryEntry)> + ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.iter.len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn read_dir() {
        let mut source = MemorySource::default();
        source.create_dir("/a1").unwrap();
        source.create_dir("/b2").unwrap();
        source.create_dir("/c3").unwrap();
        let iter = source.read_dir("/").unwrap();
        assert_eq!(iter.len(), 3);
    }

    #[test]
    fn source_file_lookup() {
        let mut source = MemorySource::default();
        assert_eq!(
            source.open("balls").err().unwrap(),
            FoldyError::DirectoryNotFound
        );

        source.open_mut("balls").unwrap();
        source.open("balls").unwrap();
        source.open("/balls").unwrap();

        source.create_dir("/asdf").unwrap();

        assert_eq!(source.open("/asdf").err().unwrap(), FoldyError::InvalidPath);

        source.open_mut("/asdf/123").unwrap();
        source.open("/asdf/123").unwrap();
        source.open("/asdf/123").unwrap();

        source.create_dir("/asdf/abc").unwrap();
        source.open_mut("/asdf/abc/fff").unwrap();
        source.open("/asdf/abc/fff").unwrap();
    }

    #[test]
    fn memory_file_stdio() {
        use std::io::Read;

        let test_data: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut file = MemoryFile::from_slice(&test_data);

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();

        assert_eq!(&buffer, &test_data);
    }

    #[test]
    fn memory_file() {
        let test_data: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        let mut test_read: [u8; 16] = [0; 16];
        let mut file = MemoryFile::default();
        file.write(&test_data);

        File::seek(&mut file, 0).unwrap();
        assert_eq!(file.position(), 0);
        File::read(&mut file, &mut test_read).unwrap();
        assert_eq!(&test_data, &test_read);
    }
}
