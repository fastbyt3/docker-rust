use std::{env, fs, os::unix::fs::chroot, path::Path, process::Stdio};

use anyhow::{Context, Result};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let command = &args[3];
    let command_args = &args[4..];

    // chrooting
    // create temp dir (with /dev/null)
    // chroot to tmpdir & set curr dir (chdir) to "/"
    // copy cmd binary (either preserve path / put in root dir)
    // run cmd
    let tmpdir = tempfile::tempdir().context("Getting a temp dir").unwrap();
    let tmpdir_dev = tmpdir.path().join("dev");
    let tmpdir_dev_null = tmpdir_dev.join("null");
    fs::create_dir(tmpdir_dev)
        .context("Creating temp dir with dev dir")
        .unwrap();
    fs::write(tmpdir_dev_null, b"")
        .context("Writing empty dev/null file")
        .unwrap();

    let cmd_bin = Path::new(&command)
        .file_name()
        .context("Get file name of command binary")?;
    fs::copy(command, tmpdir.path().join(&cmd_bin)).context("Copying cmd bin to tmp dir")?;

    chroot(&tmpdir).context("Chrooting to tmp dir").unwrap();
    env::set_current_dir("/").context("Setting curr dir (chdir) to tmp root")?;

    let output = std::process::Command::new(Path::new("/").join(&cmd_bin))
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
