Testovací aplikace jako balíček na Synology DiskStation

DSM 7.1.1-42962 Update 6


S procesorem Realtek RTD1293 SoC



## Co je to za stroj

```
uname -m                                   # → aarch64
grep platform /etc.defaults/synoinfo.conf  # → platform_name="rtd1296"
```



## Aplikace a cross‑kompilace

Cíl je aarch64-unknown-linux-musl.
Statická binárka nezávisí na staré glibc v DSM, takže odpadají problémy s kompatibilitou.

Ukázka jednoduchého HTTP serveru jen se std.


```
rustup target add aarch64-unknown-linux-musl
cargo install cargo-zigbuild      # vyžaduje nainstalovaný zig
pip install ziglang
cargo zigbuild --release --target aarch64-unknown-linux-musl
```


Druhá možnost je cargo install cross a pak cross build --release --target aarch64-unknown-linux-musl, ta ale potřebuje Docker. Do Cargo.toml se hodí přidat [profile.release] strip = true, lto = true, aby byla binárka menší.

```
docker run --rm -it \
  -v "$PWD":/home/rust/src \
  -v "$HOME/.cargo/registry":/root/.cargo/registry \
  messense/rust-musl-cross:aarch64-musl \
  cargo build --release
```


Rychlý test před balením:

```
scp -O target/aarch64-unknown-linux-musl/release/synodemo admin@nas:/tmp/
ssh admin@nas /tmp/synodemo
```



## Vytvoření balíčku (.spk)

Nejdřív z kořene repozitáře nainstaluj `cargo-generate-spk`: `cargo install --path .`

```
cargo zigbuild --release --target aarch64-unknown-linux-musl
cargo generate-spk --target aarch64-unknown-linux-musl
```

Výsledek je `target/generate-spk/synodemo-0.1.0-rtd1296.spk`. Nastavení balíčku je
v `[package.metadata.generate-spk]` v `Cargo.toml`.


## Instalace na NAS

1. DSM → Správce balíčků → Ruční instalace → vybrat `.spk`.
2. DSM upozorní na nepodepsaný balíček, to je u vlastních balíčků v pořádku.
3. Po spuštění ověřit: `curl http://nas:8085`.

Služba běží pod uživatelem balíčku (`conf/privilege`), PID a log jsou v
`/var/packages/synodemo/var/`. Pokud je zapnutý firewall DSM, je třeba povolit
port 8085.
