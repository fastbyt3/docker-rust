use std::process::Stdio;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];
    let output = std::process::Command::new(command)
        .args(command_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "Tried to run '{}' with arguments {:?}",
                command, command_args
            )
        })?;

    let std_out = std::str::from_utf8(&output.stdout)?;
    print!("{std_out}");
    let std_err = std::str::from_utf8(&output.stderr)?;
    eprint!("{}", std_err);
    std::process::exit(
        *(&output
            .status
            .code()
            .context("Attempt to read Exit code from process")
            .unwrap()),
    );
}
