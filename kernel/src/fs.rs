use alloc::vec::Vec;
use core::{mem, slice};
use uefi::fs::{File as InnerFile, FileInfo, SimpleFileSystem, FILE_MODE_READ};
use uefi::guid::{Guid, FILE_SYSTEM_GUID};
use uefi::status::{Error, Result};

use proto::Protocol;
use string::wstr;

pub struct FileSystem(pub &'static mut SimpleFileSystem);

impl Protocol<SimpleFileSystem> for FileSystem {
    fn guid() -> Guid {
        FILE_SYSTEM_GUID
    }

    fn new(inner: &'static mut SimpleFileSystem) -> Self {
        FileSystem(inner)
    }
}

impl FileSystem {
    pub fn root(&mut self) -> Result<Dir> {
        let mut interface = 0 as *mut InnerFile;
        (self.0.OpenVolume)(self.0, &mut interface)?;

        Ok(Dir(File(unsafe { &mut *interface })))
    }
}

pub struct File(pub &'static mut InnerFile);

impl File {
    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut len = buf.len();
        (self.0.Read)(self.0, &mut len, buf.as_mut_ptr())?;
        Ok(len)
    }

    pub fn read_to_end(&mut self, vec: &mut Vec<u8>) -> Result<usize> {
        let mut total = 0;
        let mut buf = [0; 8192];

        while let count = self.read(&mut buf)? {
            if count == 0 { break }

            vec.extend(&buf[.. count]);
            total += count;
        }

        Ok(total)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let mut len = buf.len();
        (self.0.Write)(self.0, &mut len, buf.as_ptr())?;
        Ok(len)
    }

    pub fn get_position(&mut self) -> Result<u64> {
        let mut pos = 0;
		(self.0.GetPosition)(self.0, &mut pos)?;
        Ok(pos)
    }

    pub fn set_position(&mut self, pos: u64) {
        (self.0.SetPosition)(self.0, pos);
    }
}

impl Drop for File {
    fn drop(&mut self) {
        let _ = (self.0.Close)(self.0);
    }
}

pub struct Dir(pub File);

impl Dir {
    pub fn open(&mut self, filename: &[u16]) -> Result<File> {
        let mut interface = 0 as *mut InnerFile;
        ((self.0).0.Open)((self.0).0, &mut interface, filename.as_ptr(), FILE_MODE_READ, 0)?;

        Ok(File(unsafe { &mut *interface }))
    }

    pub fn open_dir(&mut self, filename: &[u16]) -> Result<Dir> {
        let file = self.open(filename)?;
        Ok(Dir(file))
    }

    pub fn read(&mut self) -> Result<Option<FileInfo>> {
        let mut info = FileInfo::default();
        let buf = unsafe {
            slice::from_raw_parts_mut(
                &mut info as *mut _ as *mut u8,
                mem::size_of_val(&info)
            )
        };
        match self.0.read(buf) {
            Ok(0) => Ok(None),
            Ok(_len) => Ok(Some(info)),
            Err(err) => Err(err)
        }
    }
}

pub fn find(path: &str) -> Result<(usize, File)> {
    let wpath = wstr(path);

    for (i, mut fs) in FileSystem::all().iter_mut().enumerate() {
        let mut root = fs.root()?;
        match root.open(&wpath) {
            Ok(file) => {
                return Ok((i, file));
            },
            Err(err) => if err != Error::NotFound {
                return Err(err);
            }
        }
    }

    Err(Error::NotFound)
}

pub fn load(path: &str) -> Result<Vec<u8>> {
    let (_, mut file) = find(path)?;

    let mut data = vec![];
    let _ = file.read_to_end(&mut data)?;

    Ok(data)
}
