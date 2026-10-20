# cargo-generate-spk

[**English**](README.md) | [Česky](README.cs.md)

A Cargo helper command that generates a Synology DiskStation package (`.spk`) from a
Cargo project. It needs no Synology tooling, only Cargo. Inspired by
[cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm).

## Installation

```
cargo install --path .
```

## Usage

Run in the Cargo project you want to package:

```
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo generate-spk --target aarch64-unknown-linux-musl
```

The result is `target/generate-spk/<name>-<version>-<arch>.spk`. Use `-o` to change the
output (a file or a directory). `--target` and `--profile` must match how the binaries
were built; the command neither builds the binaries nor strips them.

DSM runs binaries without glibc, so the `*-unknown-linux-musl` targets are a good fit.
A plain `cargo build --target ...` fails on a different host architecture (the host linker cannot link foreign objects). Use [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) (`cargo install cargo-zigbuild`, needs `zig`) or configure a cross linker.

## Configuration

Settings live in the `[package.metadata.generate-spk]` table of `Cargo.toml`.
Most `INFO` fields default to values from `[package]`.

| Option | Mandatory | Default |
|---|---|---|
| `name` | no | `package.name` |
| `version` | no | `package.version` |
| `description` | no | `package.description` (one of them is required) |
| `maintainer` | no | first of `package.authors`, without the e-mail |
| `displayname` | no | `name` |
| `arch` | **yes** | array of DSM architectures, e.g. `["rtd1296"]` |
| `os_min_ver` | no | `7.0-40000` |
| `startable` | no | `false` |
| `assets` | **yes** | files put into `package.tgz`, see below |
| `scripts` | no | directory with scripts (`start-stop-status`, `postinst`, …) |
| `conf` | no | `conf` directory (`privilege`, `resource`); the default `privilege` is `run-as: package` |
| `icon`, `icon_256` | no | PNG, 72 px and 256 px |
| `info` | no | table of extra `INFO` fields, passed through unchanged |

`assets` is an array of items with `source`, `dest` and `mode` (octal string, default
`"0644"`). `dest` is relative to `/var/packages/<name>/target`. If `source` contains `*`,
`dest` must end with `/`. The `target/release/` prefix in `source` is replaced by the real
build directory according to `--target` and `--profile`.

```toml
[package.metadata.generate-spk]
displayname = "Synodemo"
arch = ["rtd1296"]
os_min_ver = "7.1-42962"
startable = true
scripts = "spk/scripts"
conf = "spk/conf"
assets = [
    { source = "target/release/synodemo", dest = "bin/synodemo", mode = "0755" },
]
```

A working example is in [`examples/synodemo`](examples/synodemo):

```
cargo install --path .      # once, from the repository root
cd examples/synodemo
rustup target add aarch64-unknown-linux-musl
cargo install cargo-zigbuild   # needs zig
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo generate-spk --target aarch64-unknown-linux-musl
```

## License

MIT, see [LICENSE](LICENSE).
