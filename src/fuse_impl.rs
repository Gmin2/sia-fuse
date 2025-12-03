use crate::storage::{FileKind, Inode, InMemoryStorage};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyCreate, ReplyData, ReplyDirectory, ReplyEmpty,
    ReplyEntry, ReplyOpen, ReplyWrite, Request,
};
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};

const TTL: Duration = Duration::from_secs(1);

pub struct SiaFuseFilesystem {
    storage: InMemoryStorage,
}

impl SiaFuseFilesystem {
    pub fn new() -> Self {
        tracing::info!("Initializing SiaFuseFilesystem");
        Self {
            storage: InMemoryStorage::new(),
        }
    }

    fn inode_to_path(&self, _ino: Inode) -> String {
        // For POC, we don't track full paths yet
        format!("inode_{}", _ino)
    }
}

impl Filesystem for SiaFuseFilesystem {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        tracing::debug!(
            "lookup(parent={}, name={})",
            parent,
            name.to_string_lossy()
        );

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        match self.storage.lookup(parent, name_str) {
            Some(attr) => {
                tracing::debug!("lookup found: ino={}", attr.ino);
                reply.entry(&TTL, &attr.to_fuser_attr(), 0);
            }
            None => {
                tracing::debug!("lookup not found");
                reply.error(libc::ENOENT);
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        tracing::debug!("getattr(ino={})", ino);

        match self.storage.get_attr(ino) {
            Some(attr) => {
                reply.attr(&TTL, &attr.to_fuser_attr());
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        tracing::debug!("read(ino={}, offset={}, size={})", ino, offset, size);

        match self.storage.read(ino, offset as usize, size as usize) {
            Some(data) => {
                tracing::debug!("read {} bytes", data.len());
                reply.data(&data);
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    fn write(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _write_flags: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyWrite,
    ) {
        tracing::debug!("write(ino={}, offset={}, len={})", ino, offset, data.len());

        match self.storage.write(ino, offset as usize, data) {
            Some(written) => {
                tracing::debug!("wrote {} bytes", written);
                reply.written(written as u32);
            }
            None => {
                reply.error(libc::ENOENT);
            }
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        tracing::debug!("readdir(ino={}, offset={})", ino, offset);

        let entries = match self.storage.read_dir(ino) {
            Some(e) => e,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        let mut current_offset = offset;

        // Add . and .. entries
        if offset == 0 {
            if reply.add(ino, 1, FileType::Directory, ".") {
                reply.ok();
                return;
            }
            current_offset += 1;
        }

        if offset <= 1 {
            if reply.add(ino, 2, FileType::Directory, "..") {
                reply.ok();
                return;
            }
            current_offset += 1;
        }

        // Add actual entries
        for (i, entry) in entries.iter().enumerate() {
            let entry_offset = i as i64 + 2; // Skip . and ..
            if entry_offset < offset {
                continue;
            }

            if reply.add(
                entry.ino,
                entry_offset + 1,
                entry.kind.to_fuser_type(),
                &entry.name,
            ) {
                break;
            }
        }

        reply.ok();
    }

    fn create(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        _flags: i32,
        reply: ReplyCreate,
    ) {
        tracing::debug!(
            "create(parent={}, name={}, mode={})",
            parent,
            name.to_string_lossy(),
            mode
        );

        let name_str = match name.to_str() {
            Some(s) => s.to_string(),
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        match self.storage.create_file(parent, name_str, mode as u16) {
            Some(attr) => {
                tracing::debug!("created file: ino={}", attr.ino);
                reply.created(&TTL, &attr.to_fuser_attr(), 0, 0, 0);
            }
            None => {
                reply.error(libc::EIO);
            }
        }
    }

    fn mkdir(
        &mut self,
        _req: &Request,
        parent: u64,
        name: &OsStr,
        mode: u32,
        _umask: u32,
        reply: ReplyEntry,
    ) {
        tracing::debug!(
            "mkdir(parent={}, name={}, mode={})",
            parent,
            name.to_string_lossy(),
            mode
        );

        let name_str = match name.to_str() {
            Some(s) => s.to_string(),
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        match self.storage.create_dir(parent, name_str, mode as u16) {
            Some(attr) => {
                tracing::debug!("created directory: ino={}", attr.ino);
                reply.entry(&TTL, &attr.to_fuser_attr(), 0);
            }
            None => {
                reply.error(libc::EIO);
            }
        }
    }

    fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        tracing::debug!("unlink(parent={}, name={})", parent, name.to_string_lossy());

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        if self.storage.unlink(parent, name_str) {
            tracing::debug!("unlinked successfully");
            reply.ok();
        } else {
            reply.error(libc::ENOENT);
        }
    }

    fn rmdir(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
        tracing::debug!("rmdir(parent={}, name={})", parent, name.to_string_lossy());

        let name_str = match name.to_str() {
            Some(s) => s,
            None => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        if self.storage.rmdir(parent, name_str) {
            tracing::debug!("removed directory successfully");
            reply.ok();
        } else {
            reply.error(libc::ENOTEMPTY);
        }
    }

    fn open(&mut self, _req: &Request, ino: u64, _flags: i32, reply: ReplyOpen) {
        tracing::debug!("open(ino={}, flags={})", ino, _flags);

        // For POC, we always allow opens
        reply.opened(0, 0);
    }

    fn release(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
        reply: ReplyEmpty,
    ) {
        tracing::debug!("release(ino={})", ino);
        reply.ok();
    }

    fn setattr(
        &mut self,
        _req: &Request,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        _atime: Option<fuser::TimeOrNow>,
        _mtime: Option<fuser::TimeOrNow>,
        _ctime: Option<std::time::SystemTime>,
        _fh: Option<u64>,
        _crtime: Option<std::time::SystemTime>,
        _chgtime: Option<std::time::SystemTime>,
        _bkuptime: Option<std::time::SystemTime>,
        _flags: Option<u32>,
        reply: ReplyAttr,
    ) {
        tracing::debug!("setattr(ino={}, size={:?})", ino, size);

        let mut attr = match self.storage.get_attr(ino) {
            Some(a) => a,
            None => {
                reply.error(libc::ENOENT);
                return;
            }
        };

        // Update attributes
        if let Some(m) = mode {
            attr.perm = m as u16;
        }
        if let Some(u) = uid {
            attr.uid = u;
        }
        if let Some(g) = gid {
            attr.gid = g;
        }
        if let Some(s) = size {
            attr.size = s;
            // Truncate file if needed
            if attr.kind == FileKind::File {
                let current_data = self.storage.read(ino, 0, usize::MAX).unwrap_or_default();
                if (s as usize) < current_data.len() {
                    let truncated = &current_data[..s as usize];
                    self.storage.write(ino, 0, truncated);
                } else if (s as usize) > current_data.len() {
                    // Extend with zeros
                    let mut extended = current_data;
                    extended.resize(s as usize, 0);
                    self.storage.write(ino, 0, &extended);
                }
            }
        }

        self.storage.set_attr(ino, attr.clone());
        reply.attr(&TTL, &attr.to_fuser_attr());
    }
}
