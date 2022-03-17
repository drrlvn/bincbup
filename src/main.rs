mod backup;
mod mount;
mod utils;

use std::ffi::OsString;
use std::path::PathBuf;

use clap::Parser;
use once_cell::sync::OnceCell;

use crate::backup::Backup;
use crate::mount::Mount;

static DRY_RUN: OnceCell<bool> = OnceCell::new();

#[derive(Parser, Debug)]
#[clap(version, about)]
struct Args {
    /// Only print what would be run
    #[clap(long)]
    dry_run: bool,

    /// Device to be mounted as source
    #[clap(long, value_name = "DEVICE")]
    source_disk: PathBuf,

    /// Path to mount source
    #[clap(long, value_name = "PATH")]
    source_mount: PathBuf,

    /// Device to be mounted as target
    #[clap(long, value_name = "DEVICE")]
    target_disk: PathBuf,

    /// Path to mount target
    #[clap(long, value_name = "PATH")]
    target_mount: PathBuf,

    /// Comma-separated list of subvolumes to sync
    #[clap(
        long,
        required = true,
        use_value_delimiter = true,
        value_name = "NAMES"
    )]
    subvolumes: Vec<OsString>,
}

fn run() -> anyhow::Result<bool> {
    let args = Args::parse();
    DRY_RUN.set(args.dry_run).unwrap();

    let source_mount = Mount::new(args.source_mount, &args.source_disk, Some("subvol=/"))?;
    let target_mount = Mount::new(args.target_mount, &args.target_disk, Some("subvol=/"))?;

    let mut backups: Vec<_> = args
        .subvolumes
        .into_iter()
        .map(|subvolume| Backup::new(subvolume, &source_mount, &target_mount))
        .collect();

    for backup in &mut backups {
        backup.prepare();
    }

    for backup in &mut backups {
        backup.replicate();
    }

    for backup in &mut backups {
        backup.finalize();
    }

    let mut success = true;
    for backup in backups {
        success &= backup.print_summary();
    }

    Ok(success)
}

fn main() {
    std::process::exit(match run() {
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
        Ok(success) => !success as i32,
    });
}
