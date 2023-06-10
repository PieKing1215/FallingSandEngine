use std::{
    ffi::OsStr,
    fs,
    io::{Read, Seek},
    path::{Path, PathBuf},
    string::ToString,
    sync::{RwLock, RwLockWriteGuard},
};

use thiserror::Error;
use zip::{read::ZipFile, result::ZipError, ZipArchive};

pub enum DirOrZip {
    Dir(PathBuf),
    Zip {
        path: PathBuf,
        zip: RwLock<ZipArchive<Box<dyn ReadSeek + Send + Sync>>>,
    },
}

pub trait ReadSeek: Read + Seek {}

impl<T: Read + Seek> ReadSeek for T {}

pub type RelativePathBuf = PathBuf;

impl DirOrZip {
    pub fn path(&self) -> &PathBuf {
        match self {
            DirOrZip::Dir(path) | DirOrZip::Zip { path, .. } => path,
        }
    }

    pub fn read_file<P: AsRef<Path>>(&self, path: P) -> Option<Vec<u8>> {
        self.file(path).and_then(|mut f| f.read().ok())
    }

    pub fn file<P: AsRef<Path>>(&self, path: P) -> Option<Box<dyn ReadEntry + '_>> {
        match self {
            DirOrZip::Dir(root) => {
                let path = root.join(path);
                path.exists().then(|| Box::new(path) as _)
            },
            DirOrZip::Zip { zip, .. } => {
                let mut wr = zip.write().unwrap();

                let path_str = path.as_ref().as_os_str().to_str().expect("Invalid path");
                if wr.by_name(path_str).is_ok() {
                    Some(Box::new(ZipFileGuard {
                        guard: wr,
                        by: ZipBy::Path(path.as_ref().to_path_buf()),
                    }) as _)
                } else {
                    None
                }
            },
        }
    }

    // TODO: make this include folders for zips
    pub fn iter_dir<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
    ) -> Box<dyn Iterator<Item = (Box<dyn ReadEntry + '_>, RelativePathBuf)> + '_> {
        match self {
            DirOrZip::Dir(root) => root.iter_dir(path),
            DirOrZip::Zip { zip, .. } => {
                let len = zip.write().unwrap().len();
                Box::new((0..len).filter_map(move |i| {
                    let mut zip = zip.write().unwrap();

                    let f = zip.by_index(i).unwrap();
                    let p = f.enclosed_name().expect("Invalid path");
                    if p.parent().unwrap() == path.as_ref() {
                        let pb = p.to_path_buf();
                        drop(f);

                        Some((
                            Box::new(ZipFileGuard { guard: zip, by: ZipBy::Index(i) }) as _,
                            pb,
                        ))
                    } else {
                        None
                    }
                }))
            },
        }
    }
}

pub trait PathBufExt {
    fn iter_dir<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
    ) -> Box<dyn Iterator<Item = (Box<dyn ReadEntry + '_>, RelativePathBuf)> + '_>;
}

impl PathBufExt for PathBuf {
    fn iter_dir<'a, P: AsRef<Path> + 'a>(
        &'a self,
        path: P,
    ) -> Box<dyn Iterator<Item = (Box<dyn ReadEntry + '_>, RelativePathBuf)> + '_> {
        Box::new(
            fs::read_dir(self.join(path))
                .into_iter()
                .flat_map(move |dir| {
                    dir.flatten()
                        .flat_map(|entry| {
                            let p = entry.path();
                            p.strip_prefix(self)
                                .map(|rel| (Box::new(p.clone()) as _, rel.into()))
                        })
                        .collect::<Vec<_>>()
                }),
        )
    }
}

pub trait ReadEntry {
    fn is_dir(&mut self) -> bool;
    fn file_stem(&mut self) -> Option<String>;
    fn extension(&mut self) -> Option<String>;
    fn read(&mut self) -> Result<Vec<u8>, ReadFileError>;
    fn read_to_string(&mut self) -> Result<String, ReadFileError>;
}

#[derive(Error, Debug)]
pub enum ReadFileError {
    #[error("io error")]
    IOError(#[from] std::io::Error),
    #[error("zip error")]
    ZipError(#[from] ZipError),
}

struct ZipFileGuard<'a> {
    guard: RwLockWriteGuard<'a, ZipArchive<Box<dyn ReadSeek + Send + Sync>>>,
    by: ZipBy,
}

enum ZipBy {
    Path(PathBuf),
    Index(usize),
}

impl ReadEntry for ZipFileGuard<'_> {
    fn file_stem(&mut self) -> Option<String> {
        let file = match &self.by {
            ZipBy::Path(p) => self
                .guard
                .by_name(p.as_os_str().to_str().expect("Invalid path")),
            ZipBy::Index(i) => self.guard.by_index(*i),
        };

        file.ok().and_then(|mut f| f.file_stem())
    }

    fn extension(&mut self) -> Option<String> {
        let file = match &self.by {
            ZipBy::Path(p) => self
                .guard
                .by_name(p.as_os_str().to_str().expect("Invalid path")),
            ZipBy::Index(i) => self.guard.by_index(*i),
        };

        file.ok().and_then(|mut f| f.extension())
    }

    fn read(&mut self) -> Result<Vec<u8>, ReadFileError> {
        let file = match &self.by {
            ZipBy::Path(p) => self
                .guard
                .by_name(p.as_os_str().to_str().expect("Invalid path")),
            ZipBy::Index(i) => self.guard.by_index(*i),
        };

        ReadEntry::read(&mut file?)
    }

    fn read_to_string(&mut self) -> Result<String, ReadFileError> {
        let file = match &self.by {
            ZipBy::Path(p) => self
                .guard
                .by_name(p.as_os_str().to_str().expect("Invalid path")),
            ZipBy::Index(i) => self.guard.by_index(*i),
        };

        ReadEntry::read_to_string(&mut file?)
    }

    fn is_dir(&mut self) -> bool {
        let file = match &self.by {
            ZipBy::Path(p) => self
                .guard
                .by_name(p.as_os_str().to_str().expect("Invalid path")),
            ZipBy::Index(i) => self.guard.by_index(*i),
        };

        file.map_or(false, |f| f.is_dir())
    }
}

impl ReadEntry for ZipFile<'_> {
    fn read(&mut self) -> Result<Vec<u8>, ReadFileError> {
        let mut buf = vec![];
        self.read_to_end(&mut buf)?;

        Ok(buf)
    }

    fn read_to_string(&mut self) -> Result<String, ReadFileError> {
        let mut buf = String::new();
        std::io::Read::read_to_string(self, &mut buf)?;

        Ok(buf)
    }

    fn extension(&mut self) -> Option<String> {
        self.enclosed_name().and_then(|n| {
            n.extension()
                .and_then(OsStr::to_str)
                .map(ToString::to_string)
        })
    }

    fn file_stem(&mut self) -> Option<String> {
        self.enclosed_name().and_then(|n| {
            n.file_stem()
                .and_then(OsStr::to_str)
                .map(ToString::to_string)
        })
    }

    fn is_dir(&mut self) -> bool {
        ZipFile::is_dir(self)
    }
}

impl ReadEntry for PathBuf {
    fn read(&mut self) -> Result<Vec<u8>, ReadFileError> {
        Ok(fs::read(self)?)
    }

    fn read_to_string(&mut self) -> Result<String, ReadFileError> {
        Ok(fs::read_to_string(self)?)
    }

    fn extension(&mut self) -> Option<String> {
        std::path::Path::extension(self)
            .and_then(OsStr::to_str)
            .map(ToString::to_string)
    }

    fn file_stem(&mut self) -> Option<String> {
        std::path::Path::file_stem(self)
            .and_then(OsStr::to_str)
            .map(ToString::to_string)
    }

    fn is_dir(&mut self) -> bool {
        std::path::Path::is_dir(self)
    }
}

#[derive(Error, Debug)]
pub enum DirOrZipError {
    #[error("path not directory or zip")]
    NotDirOrZip(PathBuf),
}

impl TryFrom<PathBuf> for DirOrZip {
    type Error = DirOrZipError;

    fn try_from(mut path: PathBuf) -> Result<Self, Self::Error> {
        if path.is_dir() {
            Ok(Self::Dir(path))
        } else if path
            .extension()
            .map_or(false, |ext| ext.eq_ignore_ascii_case("zip"))
        {
            let file = fs::File::open(&path).unwrap();
            let zip = zip::ZipArchive::new(Box::new(file) as _).unwrap();

            Ok(DirOrZip::Zip { path, zip: RwLock::new(zip) })
        } else {
            Err(DirOrZipError::NotDirOrZip(path))
        }
    }
}
