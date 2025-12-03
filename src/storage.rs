use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for inodes
pub type Inode = u64;

/// File attributes
#[derive(Debug, Clone)]
pub struct FileAttr {
    pub ino: Inode,
    pub size: u64,
    pub kind: FileKind,
    pub perm: u16,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub flags: u32,
    pub atime: DateTime<Utc>,
    pub mtime: DateTime<Utc>,
    pub ctime: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileKind {
    File,
    Directory,
}

impl FileKind {
    pub fn to_fuser_type(&self) -> fuser::FileType {
        match self {
            FileKind::File => fuser::FileType::RegularFile,
            FileKind::Directory => fuser::FileType::Directory,
        }
    }
}

impl FileAttr {
    pub fn to_fuser_attr(&self) -> fuser::FileAttr {
        let blksize = 4096;
        let blocks = (self.size + blksize - 1) / blksize;

        fuser::FileAttr {
            ino: self.ino,
            size: self.size,
            blocks,
            atime: std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(self.atime.timestamp() as u64),
            mtime: std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(self.mtime.timestamp() as u64),
            ctime: std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(self.ctime.timestamp() as u64),
            crtime: std::time::UNIX_EPOCH
                + std::time::Duration::from_secs(self.ctime.timestamp() as u64),
            kind: self.kind.to_fuser_type(),
            perm: self.perm,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev,
            blksize: blksize as u32,
            flags: self.flags,
        }
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub ino: Inode,
    pub name: String,
    pub kind: FileKind,
}

/// In-memory file data
#[derive(Debug, Clone)]
struct FileData {
    pub attr: FileAttr,
    pub content: Vec<u8>,
    pub children: Vec<DirEntry>, // Only for directories
}

/// In-memory storage backend
pub struct InMemoryStorage {
    files: Arc<RwLock<HashMap<Inode, FileData>>>,
    next_inode: Arc<RwLock<Inode>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        let mut files = HashMap::new();
        let now = Utc::now();

        // Create root directory (inode 1)
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            kind: FileKind::Directory,
            perm: 0o755,
            nlink: 2,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            flags: 0,
            atime: now,
            mtime: now,
            ctime: now,
        };

        files.insert(
            1,
            FileData {
                attr: root_attr,
                content: Vec::new(),
                children: Vec::new(),
            },
        );

        Self {
            files: Arc::new(RwLock::new(files)),
            next_inode: Arc::new(RwLock::new(2)),
        }
    }

    /// Allocate a new inode
    pub fn allocate_inode(&self) -> Inode {
        let mut next = self.next_inode.write();
        let ino = *next;
        *next += 1;
        ino
    }

    /// Get file attributes
    pub fn get_attr(&self, ino: Inode) -> Option<FileAttr> {
        self.files.read().get(&ino).map(|f| f.attr.clone())
    }

    /// Set file attributes
    pub fn set_attr(&self, ino: Inode, attr: FileAttr) -> bool {
        if let Some(file) = self.files.write().get_mut(&ino) {
            file.attr = attr;
            true
        } else {
            false
        }
    }

    /// Read file content
    pub fn read(&self, ino: Inode, offset: usize, size: usize) -> Option<Vec<u8>> {
        self.files.read().get(&ino).map(|f| {
            let end = std::cmp::min(offset + size, f.content.len());
            if offset >= f.content.len() {
                Vec::new()
            } else {
                f.content[offset..end].to_vec()
            }
        })
    }

    /// Write file content
    pub fn write(&self, ino: Inode, offset: usize, data: &[u8]) -> Option<usize> {
        let mut files = self.files.write();
        if let Some(file) = files.get_mut(&ino) {
            let end = offset + data.len();

            // Extend if necessary
            if end > file.content.len() {
                file.content.resize(end, 0);
            }

            // Write data
            file.content[offset..end].copy_from_slice(data);

            // Update size and mtime
            file.attr.size = file.content.len() as u64;
            file.attr.mtime = Utc::now();

            Some(data.len())
        } else {
            None
        }
    }

    /// Create a new file
    pub fn create_file(&self, parent: Inode, name: String, perm: u16) -> Option<FileAttr> {
        let ino = self.allocate_inode();
        let now = Utc::now();

        let attr = FileAttr {
            ino,
            size: 0,
            kind: FileKind::File,
            perm,
            nlink: 1,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            flags: 0,
            atime: now,
            mtime: now,
            ctime: now,
        };

        let mut files = self.files.write();

        // Add file
        files.insert(
            ino,
            FileData {
                attr: attr.clone(),
                content: Vec::new(),
                children: Vec::new(),
            },
        );

        // Add to parent directory
        if let Some(parent_file) = files.get_mut(&parent) {
            parent_file.children.push(DirEntry {
                ino,
                name,
                kind: FileKind::File,
            });
            parent_file.attr.mtime = now;
        }

        Some(attr)
    }

    /// Create a new directory
    pub fn create_dir(&self, parent: Inode, name: String, perm: u16) -> Option<FileAttr> {
        let ino = self.allocate_inode();
        let now = Utc::now();

        let attr = FileAttr {
            ino,
            size: 0,
            kind: FileKind::Directory,
            perm,
            nlink: 2,
            uid: unsafe { libc::getuid() },
            gid: unsafe { libc::getgid() },
            rdev: 0,
            flags: 0,
            atime: now,
            mtime: now,
            ctime: now,
        };

        let mut files = self.files.write();

        // Add directory
        files.insert(
            ino,
            FileData {
                attr: attr.clone(),
                content: Vec::new(),
                children: Vec::new(),
            },
        );

        // Add to parent directory
        if let Some(parent_file) = files.get_mut(&parent) {
            parent_file.children.push(DirEntry {
                ino,
                name,
                kind: FileKind::Directory,
            });
            parent_file.attr.mtime = now;
            parent_file.attr.nlink += 1;
        }

        Some(attr)
    }

    /// List directory contents
    pub fn read_dir(&self, ino: Inode) -> Option<Vec<DirEntry>> {
        self.files.read().get(&ino).map(|f| f.children.clone())
    }

    /// Look up a file by name in a directory
    pub fn lookup(&self, parent: Inode, name: &str) -> Option<FileAttr> {
        self.files
            .read()
            .get(&parent)
            .and_then(|f| f.children.iter().find(|e| e.name == name))
            .and_then(|entry| self.files.read().get(&entry.ino).map(|f| f.attr.clone()))
    }

    /// Remove a file
    pub fn unlink(&self, parent: Inode, name: &str) -> bool {
        let mut files = self.files.write();

        // Find the file in parent's children
        if let Some(parent_file) = files.get_mut(&parent) {
            if let Some(pos) = parent_file
                .children
                .iter()
                .position(|e| e.name == name && e.kind == FileKind::File)
            {
                let ino = parent_file.children[pos].ino;
                parent_file.children.remove(pos);
                parent_file.attr.mtime = Utc::now();

                // Remove the file
                files.remove(&ino);
                return true;
            }
        }

        false
    }

    /// Remove a directory
    pub fn rmdir(&self, parent: Inode, name: &str) -> bool {
        let mut files = self.files.write();

        // Find the directory in parent's children
        if let Some(parent_file) = files.get_mut(&parent) {
            if let Some(pos) = parent_file
                .children
                .iter()
                .position(|e| e.name == name && e.kind == FileKind::Directory)
            {
                let ino = parent_file.children[pos].ino;

                // Check if directory is empty
                if let Some(dir) = files.get(&ino) {
                    if !dir.children.is_empty() {
                        return false; // Directory not empty
                    }
                }

                parent_file.children.remove(pos);
                parent_file.attr.mtime = Utc::now();
                parent_file.attr.nlink -= 1;

                // Remove the directory
                files.remove(&ino);
                return true;
            }
        }

        false
    }
}
