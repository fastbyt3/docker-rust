# DIY Docker Rust

## Executing a program

```bash
mydocker run ubuntu:latest /usr/local/bin/docker-explorer echo hey
```

## Wireup stdout and stderr

- [Rust STDIO ref](https://doc.rust-lang.org/std/process/struct.Stdio.html)

## Handle exit codes

- if program exits with code `1` => our program should exit with code 1
