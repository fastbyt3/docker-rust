# DIY Docker Rust

## Executing a program

```bash
mydocker run ubuntu:latest /usr/local/bin/docker-explorer echo hey
```

## Wireup stdout and stderr

- [Rust STDIO ref](https://doc.rust-lang.org/std/process/struct.Stdio.html)

## Handle exit codes

- if program exits with code `1` => our program should exit with code 1

## Filesystem isolation

- Using [`chroot`](https://en.wikipedia.org/wiki/Chroot) ensure program doesnt have access to host files
- create an empty dir and `chroot` into it (also copy binary)
- [Rust ref: fs::chroot](https://doc.rust-lang.org/std/os/unix/fs/fn.chroot.html)

```rust
fs::chroot("/sandbox")?;
std::env::set_current_dir("/")?;
// continue working in sandbox
```

- we also need to copy over the binary to the new temp folder which will be the ROOT for the child proc

- NOTE: Docker has replaced CHROOT with PIVOT-ROOT for [security reasons](https://tbhaxor.com/pivot-root-vs-chroot-for-containers/)
- using pivot-root ref:
	- https://gist.github.com/penumbra23/df65aaf3d5807c85b62b05608a8f30bd
    - https://docs.rs/nix/latest/nix/unistd/fn.pivot_root.html

### Pivot root

- given a new root and subdir of current root pivot-root moves root(of child process) to subdir and mounts that as new root point
- then we unmount the old root and leave the newly created root mount point

### Process isolation

- guarding the process tree
- using [**PID namespaces**](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html) we create an isolated process tree for the child process so that it cannot view/interact with host processes
- the child process must have PID = 1

### Fetching from docker registry

- Fetch from [docker registry](https://docs.docker.com/registry/) the contents of public images in docker hub then exec cmd with it
- steps:
    - auth
    - fetch image manifest
    - pull layers of img and extract to chroot dir
 
- base url: `registry.hub.docker.com`

- cmd syntax: `mydocker run ubuntu:latest /bin/echo hey`

- when interacting with registry API
    - prepend `library/` to img names

- Using [JWT authentication](https://distribution.github.io/distribution/spec/auth/jwt/)

