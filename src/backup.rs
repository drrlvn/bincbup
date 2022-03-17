use std::{
    ffi::OsString,
    path::PathBuf,
    process::{Command, Stdio},
};

use crate::mount::Mount;

struct Subvolume {
    path: PathBuf,
    base: PathBuf,
    snapshot: PathBuf,
}

impl Subvolume {
    fn finalize(self: &mut Self) -> anyhow::Result<()> {
        let mut cmd = Command::new("btrfs");
        cmd.args(["subvolume", "delete"]);
        cmd.arg(&self.base);
        crate::utils::exec(cmd)?;

        println!(
            "mv {} {}",
            self.snapshot.to_string_lossy(),
            self.base.to_string_lossy()
        );

        if *super::DRY_RUN.get().unwrap() {
            return Ok(());
        }

        Ok(std::fs::rename(&self.snapshot, &self.base)?)
    }
}

enum State {
    Failed(anyhow::Error),
    InProgress {
        source: Subvolume,
        target: Subvolume,
    },
}

impl State {
    fn finalize(self: &mut Self) -> anyhow::Result<()> {
        match self {
            Self::Failed(_) => Ok(()),
            Self::InProgress { source, target } => {
                target.finalize()?;
                source.finalize()
            }
        }
    }

    fn replicate(self: &mut Self) -> anyhow::Result<()> {
        match &self {
            State::Failed(_) => Ok(()),
            State::InProgress { source, target, .. } => {
                let mut send_command = Command::new("btrfs");
                send_command
                    .args(["send", "-p"])
                    .arg(&source.base)
                    .arg(&source.snapshot);
                let mut receive_command = Command::new("btrfs");
                receive_command.arg("receive").arg(&target.path);

                println!(
                    "{} | {}",
                    crate::utils::format_command(&send_command),
                    crate::utils::format_command(&receive_command)
                );

                if *super::DRY_RUN.get().unwrap() {
                    return Ok(());
                }

                let mut send_command = send_command.stdout(Stdio::piped()).spawn()?;

                let status = receive_command
                    .stdin(Stdio::from(send_command.stdout.take().unwrap()))
                    .status()?;
                if !status.success() {
                    let _ = send_command.wait();
                    anyhow::bail!("Receive command failed");
                }

                let status = send_command.wait()?;
                if !status.success() {
                    anyhow::bail!("Send command failed");
                }

                Ok(())
            }
        }
    }
}

pub struct Backup {
    name: OsString,
    state: State,
}

impl Backup {
    pub fn new(name: OsString, source_mount: &Mount, target_mount: &Mount) -> Self {
        let source_base = {
            let mut p = source_mount.0.join("snapshots");
            p.push(&name);
            p
        };
        let target_base = target_mount.0.join(&name);
        Self {
            state: State::InProgress {
                source: Subvolume {
                    path: source_mount.0.join(&name),
                    snapshot: source_base.with_file_name({
                        let mut s = name.clone();
                        s.push("-new");
                        s
                    }),
                    base: source_base,
                },
                target: Subvolume {
                    path: target_mount.0.to_owned(),
                    snapshot: target_base.with_file_name({
                        let mut s = name.clone();
                        s.push("-new");
                        s
                    }),
                    base: target_base,
                },
            },
            name,
        }
    }

    fn exec(self: &mut Self, cmd: Command) {
        match &self.state {
            State::Failed(_) => (),
            State::InProgress { .. } => {
                if let Err(err) = crate::utils::exec(cmd) {
                    self.state = State::Failed(err);
                    return;
                }
            }
        }
    }

    pub fn prepare(self: &mut Self) {
        match &self.state {
            State::Failed(_) => (),
            State::InProgress { source, .. } => {
                let mut cmd = Command::new("btrfs");
                cmd.args(["subvolume", "snapshot", "-r"])
                    .arg(&source.path)
                    .arg(&source.snapshot);
                self.exec(cmd);
            }
        }
    }

    pub fn replicate(self: &mut Self) {
        if let Err(err) = self.state.replicate() {
            self.state = State::Failed(err);
        }
    }

    pub fn finalize(self: &mut Self) {
        if let Err(err) = self.state.finalize() {
            self.state = State::Failed(err);
        }
    }

    pub fn print_summary(self: Self) -> bool {
        print!("Backup {}: ", self.name.to_string_lossy());
        let success = match &self.state {
            State::Failed(err) => {
                print!("{}", err);
                false
            }
            State::InProgress { .. } => {
                print!("Success");
                true
            }
        };
        if *super::DRY_RUN.get().unwrap() {
            print!(" (DRY RUN)");
        }
        println!();
        success
    }
}
