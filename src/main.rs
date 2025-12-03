use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod fuse_impl;
mod storage;

use fuse_impl::SiaFuseFilesystem;

#[derive(Parser)]
#[command(name = "sia-fuse")]
#[command(about = "Native FUSE filesystem driver for Sia network", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Mount Sia filesystem
    Mount {
        /// Mount point directory
        mountpoint: PathBuf,

        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,

        /// Allow other users to access the filesystem
        #[arg(long)]
        allow_other: bool,
    },

    /// Initialize configuration
    Init {
        /// Configuration directory
        #[arg(short, long, default_value = "~/.config/sia-fuse")]
        config_dir: PathBuf,
    },

    /// Show version information
    Version,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Mount {
            mountpoint,
            debug,
            allow_other,
        } => {
            // Initialize logging
            let filter = if debug {
                EnvFilter::new("sia_fuse_rs=debug")
            } else {
                EnvFilter::new("sia_fuse_rs=info")
            };

            tracing_subscriber::registry()
                .with(fmt::layer())
                .with(filter)
                .init();

            tracing::info!("Starting sia-fuse v{}", env!("CARGO_PKG_VERSION"));
            tracing::info!("Mounting at: {}", mountpoint.display());

            // Create mountpoint if it doesn't exist
            if !mountpoint.exists() {
                std::fs::create_dir_all(&mountpoint)?;
                tracing::info!("Created mount point directory");
            }

            // Create filesystem
            let fs = SiaFuseFilesystem::new();

            // Mount options
            let mut options = vec![
                fuser::MountOption::FSName("sia-fuse".to_string()),
                fuser::MountOption::RW,
                fuser::MountOption::AutoUnmount,
            ];

            if allow_other {
                options.push(fuser::MountOption::AllowOther);
            }

            tracing::info!("Mounting filesystem...");
            tracing::info!("Press Ctrl+C to unmount");

            // Mount the filesystem (this blocks until unmount)
            fuser::mount2(fs, mountpoint, &options)?;

            tracing::info!("Filesystem unmounted");
        }

        Commands::Init { config_dir } => {
            println!("Initializing sia-fuse configuration...");
            println!("Config directory: {}", config_dir.display());

            // Create config directory
            std::fs::create_dir_all(&config_dir)?;

            println!();
            println!("Configuration initialized successfully!");
            println!();
            println!("Next steps:");
            println!("  1. Mount the filesystem:");
            println!("     sia-fuse mount ~/sia");
            println!();
            println!("  2. Use it like a normal folder:");
            println!("     echo 'Hello Sia!' > ~/sia/test.txt");
            println!("     cat ~/sia/test.txt");
        }

        Commands::Version => {
            println!("sia-fuse v{}", env!("CARGO_PKG_VERSION"));
            println!("A native FUSE filesystem driver for Sia network");
        }
    }

    Ok(())
}
