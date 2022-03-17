use std::process::Command;

pub fn exec(mut cmd: Command) -> anyhow::Result<()> {
    println!("{}", format_command(&cmd));

    if *super::DRY_RUN.get().unwrap() {
        return Ok(());
    }

    match cmd.status()?.code() {
        None => anyhow::bail!("Command failed with signal"),
        Some(code) if code != 0 => anyhow::bail!("Command failed with {code}"),
        _ => Ok(()),
    }
}

pub fn format_command(cmd: &Command) -> String {
    format!(
        "{} {}",
        cmd.get_program().to_string_lossy(),
        cmd.get_args()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" "),
    )
}
