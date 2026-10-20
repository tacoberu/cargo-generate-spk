# cargo-generate-spk

[English](README.md) | **Česky**

Pomocný příkaz Cargo, který z Cargo projektu vytvoří balíček pro Synology
DiskStation (`.spk`). Nepotřebuje žádné nástroje Synology, jen Cargo. Inspirováno
[cargo-generate-rpm](https://github.com/cat-in-136/cargo-generate-rpm).

## Instalace

```
cargo install --path .
```

## Použití

Spouští se v Cargo projektu, který chceš zabalit:

```
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo generate-spk --target aarch64-unknown-linux-musl
```

Výsledek je `target/generate-spk/<název>-<verze>-<arch>.spk`. Výstup lze změnit přepínačem `-o`
(soubor nebo adresář). Přepínače `--target` a `--profile` musí odpovídat tomu, jak byla
binárka přeložena; příkaz binárky nepřekládá ani neodstraňuje z nich symboly.

Na DSM běží binárky bez glibc, proto se pro ně hodí cíl `*-unknown-linux-musl`.
Samotný `cargo build --target ...` na stroji s jinou architekturou selže (systémový linker cizí objekty nesleje). Použij [`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) (`cargo install cargo-zigbuild`, vyžaduje `zig`) nebo nastav cross linker.

## Konfigurace

Nastavení je v `Cargo.toml`, v tabulce `[package.metadata.generate-spk]`.
Pole `INFO` se z velké části berou z `[package]`.

| Volba | Povinná | Výchozí |
|---|---|---|
| `name` | ne | `package.name` |
| `version` | ne | `package.version` |
| `description` | ne | `package.description` (jedno z nich je nutné) |
| `maintainer` | ne | první z `package.authors` bez e-mailu |
| `displayname` | ne | `name` |
| `arch` | **ano** | pole architektur DSM, např. `["rtd1296"]` |
| `os_min_ver` | ne | `7.0-40000` |
| `startable` | ne | `false` |
| `assets` | **ano** | soubory v `package.tgz`, viz níže |
| `scripts` | ne | adresář se skripty (`start-stop-status`, `postinst`, …) |
| `conf` | ne | adresář `conf` (`privilege`, `resource`); výchozí `privilege` je `run-as: package` |
| `icon`, `icon_256` | ne | PNG 72 px a 256 px |
| `info` | ne | tabulka dalších polí `INFO`, přenesou se beze změny |

`assets` je pole s položkami `source`, `dest` a `mode` (osmičkově, výchozí `"0644"`).
`dest` je relativní k `/var/packages/<název>/target`. Pokud `source` obsahuje `*`, musí
`dest` končit `/`. Předpona `target/release/` ve `source` se nahradí skutečným adresářem
sestavení podle `--target` a `--profile`.

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

Funkční ukázka je v [`examples/synodemo`](examples/synodemo):

```
cargo install --path .      # jednou, z kořene repozitáře
cd examples/synodemo
rustup target add aarch64-unknown-linux-musl
cargo install cargo-zigbuild   # vyžaduje zig
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo generate-spk --target aarch64-unknown-linux-musl
```

## Licence

MIT, viz [LICENSE](LICENSE).
