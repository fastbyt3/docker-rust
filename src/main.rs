use std::{env, fs, os::unix::fs::chroot, path::Path, process::Stdio};

use anyhow::{Context, Result};
use nix::sched::{unshare, CloneFlags};
use reqwest::Client;

#[tokio::main()]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let image_string = &args[2];
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

    download_image(image_string, tmpdir.path()).await?;

    let cmd_bin = Path::new(&command)
        .file_name()
        .context("Get file name of command binary")?;
    fs::copy(command, tmpdir.path().join(&cmd_bin)).context("Copying cmd bin to tmp dir")?;

    chroot(&tmpdir).context("Chrooting to tmp dir").unwrap();
    env::set_current_dir("/").context("Setting curr dir (chdir) to tmp root")?;

    // unsafe { libc::unshare(libc::CLONE_NEWPID) };
    unshare(CloneFlags::CLONE_NEWPID).context("Creating new process namespace")?;

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

async fn download_image(image: &str, tmpdir: &Path) -> Result<()> {
    let mut image_split = image.split(":");
    let image_name = image_split.next().context("Parsing image name").unwrap();
    let image_tag = image_split.next().unwrap_or("latest");

    let http_client = reqwest::Client::new();
    let auth_token = get_token(image_name).await?;
    let arch = "amd64";
    let digest = get_digest(&http_client, image_name, image_tag, &auth_token, arch).await?;
    let layers = get_layers(&http_client, image_name, &auth_token, &digest).await?;

    for layer in layers {
        let img_blob = http_client
            .get(format!(
                "https://registry.hub.docker.com/v2/library/{}/blobs/{}",
                image_name, layer.digest
            ))
            .header("Authorization", format!("Bearer: {}", auth_token))
            .send()
            .await?
            .bytes()
            .await?;

        println!("IMAGE BLOB: {img_blob:?}");

        let tar = flate2::read::GzDecoder::new(img_blob.as_ref());
        tar::Archive::new(tar)
            .unpack(tmpdir)
            .context("unpack tar contents")?;
    }

    return Ok(());
}

#[derive(serde::Deserialize, Debug)]
struct RegistryAuthResponse {
    token: String,
}

async fn get_token(image: &str) -> Result<String> {
    let auth_resp = reqwest::get(format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:library/{}:pull",
        image
    ))
    .await?
    .json::<RegistryAuthResponse>()
    .await?;

    Ok(auth_resp.token)
}

#[derive(serde::Deserialize, Debug)]
struct ImageManifest {
    manifests: Vec<Manifest>,
}

#[derive(serde::Deserialize, Debug)]
struct Manifest {
    digest: String,
    platform: Platform,
}

#[derive(serde::Deserialize, Debug)]
struct Platform {
    architecture: String,
}

async fn get_digest(
    client: &Client,
    image: &str,
    tag: &str,
    token: &str,
    arch: &str,
) -> Result<String> {
    let manifest = client
        .get(format!(
            "https://registry.hub.docker.com/v2/library/{}/manifests/{}",
            image, tag
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.list.v2+json",
        )
        .send()
        .await?
        .json::<ImageManifest>()
        .await?;

    let digest = &manifest
        .manifests
        .iter()
        .find(|m| m.platform.architecture == arch)
        .context("Couldn't find arch digest")?
        .digest;

    Ok(digest.to_owned())
}

#[derive(serde::Deserialize, Debug)]
struct ImageLayerDetails {
    layers: Vec<Layer>,
}

#[derive(serde::Deserialize, Debug)]
struct Layer {
    digest: String,
}

async fn get_layers(client: &Client, image: &str, token: &str, digest: &str) -> Result<Vec<Layer>> {
    let layer_details = client
        .get(format!(
            "https://registry.hub.docker.com/v2/library/{}/manifests/{}",
            image, digest
        ))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.oci.image.manifest.v1+json")
        .send()
        .await?
        .json::<ImageLayerDetails>()
        .await?;
    Ok(layer_details.layers)
}
