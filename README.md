# sia-fuse-rs

A native FUSE (Filesystem in Userspace) driver for the Sia decentralized storage network, written in Rust.

## Project Status: POC (Month 1 Milestone)

This is a Proof of Concept implementation demonstrating **Month 1 deliverables**:
-  Rust project scaffolding with `fuser` FUSE library
-  FUSE mount/unmount functionality
-  In-memory file storage backend
-  Core FUSE operations: read, write, readdir, create, mkdir, unlink, rmdir
-  Basic CLI (`sia-fuse mount`, `sia-fuse init`)

## Features (POC)

- **Native FUSE Implementation**: Pure Rust using the `fuser` crate
- **In-Memory Storage**: Files stored in RAM (Month 2 will add Sia network integration)
- **Standard POSIX Operations**: Works with any Unix tool (`cp`, `ls`, `vim`, etc.)
- **Multi-Threading Safe**: Uses `parking_lot` for efficient locking

## Building

```bash
# Build in release mode
cargo build --release

# The binary will be at: target/release/sia-fuse
```

## Usage

### 1. Initialize Configuration

```bash
./target/release/sia-fuse init
```

### 2. Mount the Filesystem

```bash
# Create a mount point
mkdir -p ~/sia

# Mount the filesystem
./target/release/sia-fuse mount ~/sia
```

The filesystem will remain mounted until you press Ctrl+C.

### 3. Use It Like a Normal Folder

In another terminal:

```bash
# Create a file
echo "Hello Sia!" > ~/sia/test.txt

# Read it back
cat ~/sia/test.txt

# Create a directory
mkdir ~/sia/documents

# Copy files
cp ~/Documents/*.pdf ~/sia/documents/

# List files
ls -lh ~/sia/

# Remove files
rm ~/sia/test.txt
rmdir ~/sia/documents
```

### 4. Unmount

Press `Ctrl+C` in the terminal running `sia-fuse mount`, or use:

```bash
# On Linux
fusermount -u ~/sia

# On macOS
umount ~/sia
```

## Command-Line Options

```bash
# Mount with debug logging
./target/release/sia-fuse mount ~/sia --debug

# Allow other users to access
./target/release/sia-fuse mount ~/sia --allow-other

# Show version
./target/release/sia-fuse version
```

## Testing

```bash
# Run unit tests
cargo test

# Test manual operations
./target/release/sia-fuse mount ~/sia --debug

# In another terminal:
cd ~/sia
touch test.txt
echo "data" > test.txt
cat test.txt
ls -la
rm test.txt
```

## Architecture

```
┌─────────────────────────────────────┐
│   User Applications                 │
│   (vim, cp, ls, cat, etc.)          │
└─────────────────┬───────────────────┘
                  │ POSIX I/O
┌─────────────────▼───────────────────┐
│   FUSE Kernel Module                │
└─────────────────┬───────────────────┘
                  │ libfuse protocol
┌─────────────────▼───────────────────┐
│   sia-fuse (Rust)                   │
│   ┌─────────────────────────────┐   │
│   │ fuse_impl.rs                │   │
│   │ - FUSE operation handlers   │   │
│   └─────────────┬───────────────┘   │
│                 │                    │
│   ┌─────────────▼───────────────┐   │
│   │ storage.rs                  │   │
│   │ - In-memory HashMap         │   │
│   │ - File attributes           │   │
│   │ - Directory entries         │   │
│   └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

## Current Limitations (POC)

- **In-Memory Only**: Files are not persisted across restarts
- **No Sia Integration**: Month 2 will add `indexd` SDK integration
- **No Caching**: Month 3 will add SQLite metadata cache + LRU data cache
- **No Authentication**: Month 2 will add app key authentication
- **Single-Threaded FUSE**: Operations are serialized (Rust safety guarantees thread-safety)

## Roadmap

### Month 2: Sia Network Integration
- [ ] Integrate `sia-sdk-rs` / `indexd` client
- [ ] Upload pipeline: encrypt → Reed-Solomon encode → upload 30 shards
- [ ] Download pipeline: fetch 10+ shards → decode → decrypt
- [ ] Range request support
- [ ] Object metadata storage

### Month 3: Performance Optimization
- [ ] SQLite path manager (instant directory listings)
- [ ] LRU data cache (configurable size)
- [ ] Prefetch engine for sequential reads
- [ ] Background sync worker
- [ ] Video streaming support

### Month 4: Production Release
- [ ] Enhanced CLI (`sia-fuse init/mount/status/sync`)
- [ ] Linux + macOS binaries
- [ ] Error recovery (reconnect, auth renewal)
- [ ] Comprehensive documentation
- [ ] v1.0.0 release

## Dependencies

- **fuser**: Pure Rust FUSE library
- **tokio**: Async runtime (for future network operations)
- **clap**: CLI argument parsing
- **tracing**: Structured logging
- **parking_lot**: Efficient synchronization primitives
- **chrono**: Date/time handling

## Contributing

This is a grant-funded project for the Sia Foundation. See the [grant proposal](../proposal.md) for details.

## License

MIT License - See LICENSE file for details.

## Authors

- Gmin2 - Lead Developer
- ItsMoh - Sia Integration Specialist

## Acknowledgments

- Sia Foundation for grant funding
- `fuser` crate maintainers for excellent FUSE bindings
- Sia community for feedback and support
