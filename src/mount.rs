use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub struct Mount(pub PathBuf);

impl Mount {
    pub fn new(mount: PathBuf, disk: &Path, option: Option<&str>) -> anyhow::Result<Self> {
        let mut cmd = Command::new("mount");
        if let Some(opt) = option {
            cmd.args(["-o", opt]);
        }
        cmd.args([disk, &mount]);
        crate::utils::exec(cmd)?;
        Ok(Mount(mount))
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        let mut cmd = Command::new("umount");
        cmd.arg(&self.0);
        let _ = crate::utils::exec(cmd);
    }
}
