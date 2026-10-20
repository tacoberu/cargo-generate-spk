use crate::build_target::BuildTarget;
use crate::config::{Asset, Config};
use crate::error::Error;
use flate2::{Compression, write::GzEncoder};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::{Builder, Header};

const DEFAULT_PRIVILEGE: &str = "{\n\t\"defaults\": {\n\t\t\"run-as\": \"package\"\n\t}\n}\n";

fn mtime() -> u64 {
	std::env::var("SOURCE_DATE_EPOCH")
		.ok()
		.and_then(|s| s.parse().ok())
		.unwrap_or(0)
}

fn header(size: u64, mode: u32) -> Header {
	let mut h = Header::new_gnu();
	h.set_size(size);
	h.set_mode(mode);
	h.set_uid(0);
	h.set_gid(0);
	h.set_mtime(mtime());
	h
}

fn append_bytes<W: Write>(
	tar: &mut Builder<W>,
	path: &str,
	data: &[u8],
	mode: u32,
) -> Result<(), Error> {
	tar.append_data(&mut header(data.len() as u64, mode), path, data)?;
	Ok(())
}

fn append_file<W: Write>(
	tar: &mut Builder<W>,
	path: &str,
	src: &Path,
	mode: u32,
) -> Result<(), Error> {
	let data = fs::read(src).map_err(|e| Error::FileIo(src.to_path_buf(), e))?;
	append_bytes(tar, path, &data, mode)
}

fn append_dir<W: Write>(tar: &mut Builder<W>, path: &str) -> Result<(), Error> {
	let mut h = header(0, 0o755);
	h.set_entry_type(tar::EntryType::Directory);
	tar.append_data(&mut h, path, std::io::empty())?;
	Ok(())
}

/// Adds a directory tree under `prefix`; files get `mode`. Sorted, so the output is reproducible.
fn append_tree<W: Write>(
	tar: &mut Builder<W>,
	prefix: &str,
	dir: &Path,
	mode: u32,
) -> Result<(), Error> {
	append_dir(tar, prefix)?;
	let mut entries: Vec<_> = fs::read_dir(dir)
		.map_err(|e| Error::FileIo(dir.to_path_buf(), e))?
		.collect::<Result<_, _>>()?;
	entries.sort_by_key(|e| e.file_name());
	for e in entries {
		let path = format!("{prefix}/{}", e.file_name().to_string_lossy());
		if e.file_type()?.is_dir() {
			append_tree(tar, &path, &e.path(), mode)?;
		} else {
			append_file(tar, &path, &e.path(), mode)?;
		}
	}
	Ok(())
}

/// Files an asset stands for, as (path inside package, source file).
fn expand_asset(
	config: &Config,
	target: &BuildTarget,
	asset: &Asset,
) -> Result<Vec<(String, PathBuf)>, Error> {
	let source = target.resolve(&config.project_dir, &asset.source);
	let dest = asset.dest.trim_start_matches('/');
	if !asset.source.contains('*') {
		let name = source
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_default();
		let path = if dest.ends_with('/') {
			format!("{dest}{name}")
		} else {
			dest.to_string()
		};
		return Ok(vec![(path, source)]);
	}
	if !dest.ends_with('/') {
		return Err(Error::Config(format!(
			"dest of {:?} must be a directory ending with `/`",
			asset.source
		)));
	}
	let mut files = Vec::new();
	for path in glob::glob(&source.to_string_lossy())? {
		let path = path.map_err(|e| Error::FileIo(e.path().to_path_buf(), e.into()))?;
		if path.is_file() {
			let name = path.file_name().unwrap().to_string_lossy();
			files.push((format!("{dest}{name}"), path));
		}
	}
	if files.is_empty() {
		return Err(Error::Config(format!("no file matches {:?}", asset.source)));
	}
	Ok(files)
}

/// `package.tgz`: the files installed to `/var/packages/<name>/target`.
fn package_tgz(config: &Config, target: &BuildTarget) -> Result<Vec<u8>, Error> {
	let mut tar = Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
	for asset in &config.assets {
		for (path, source) in expand_asset(config, target, asset)? {
			append_file(&mut tar, &path, &source, asset.mode)?;
		}
	}
	Ok(tar.into_inner()?.finish()?)
}

pub fn build(config: &Config, target: &BuildTarget, output: &Path) -> Result<(), Error> {
	let project = |p: &Path| config.project_dir.join(p);
	let mut tar = Builder::new(Vec::new());

	append_bytes(&mut tar, "INFO", config.info_file().as_bytes(), 0o644)?;
	append_bytes(
		&mut tar,
		"package.tgz",
		&package_tgz(config, target)?,
		0o644,
	)?;
	if let Some(dir) = &config.scripts {
		append_tree(&mut tar, "scripts", &project(dir), 0o755)?;
	}
	match &config.conf {
		Some(dir) => append_tree(&mut tar, "conf", &project(dir), 0o644)?,
		None => {
			append_dir(&mut tar, "conf")?;
			append_bytes(
				&mut tar,
				"conf/privilege",
				DEFAULT_PRIVILEGE.as_bytes(),
				0o644,
			)?;
		}
	}
	if let Some(icon) = &config.icon {
		append_file(&mut tar, "PACKAGE_ICON.PNG", &project(icon), 0o644)?;
	}
	if let Some(icon) = &config.icon_256 {
		append_file(&mut tar, "PACKAGE_ICON_256.PNG", &project(icon), 0o644)?;
	}

	if let Some(parent) = output.parent() {
		fs::create_dir_all(parent).map_err(|e| Error::FileIo(parent.to_path_buf(), e))?;
	}
	fs::write(output, tar.into_inner()?).map_err(|e| Error::FileIo(output.to_path_buf(), e))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cli::Cli;
	use std::io::Read;

	fn entries(spk: &Path) -> Vec<(String, u32)> {
		let mut ar = tar::Archive::new(fs::File::open(spk).unwrap());
		ar.entries()
			.unwrap()
			.map(|e| {
				let e = e.unwrap();
				(
					e.path().unwrap().to_string_lossy().into_owned(),
					e.header().mode().unwrap(),
				)
			})
			.collect()
	}

	#[test]
	fn builds_spk() {
		let dir = tempfile::tempdir().unwrap();
		let root = dir.path();
		fs::create_dir_all(root.join("target/release")).unwrap();
		fs::write(root.join("target/release/demo"), "binary").unwrap();
		fs::create_dir_all(root.join("spk/scripts")).unwrap();
		fs::write(root.join("spk/scripts/start-stop-status"), "#!/bin/sh\n").unwrap();
		let manifest = crate::config::tests::write_manifest(
			root,
			"arch = [\"rtd1296\"]\nscripts = \"spk/scripts\"\n\
			 assets = [{ source = \"target/release/demo\", dest = \"bin/demo\", mode = \"0755\" }]\n",
		);

		let config = Config::new(&manifest).unwrap();
		let target = BuildTarget::new(
			&Cli {
				profile: "release".into(),
				..Cli::default()
			},
			root,
		);
		let out = root.join("out.spk");
		build(&config, &target, &out).unwrap();

		let names: Vec<_> = entries(&out).into_iter().collect();
		assert!(names.contains(&("INFO".into(), 0o644)));
		assert!(names.contains(&("scripts/start-stop-status".into(), 0o755)));
		assert!(names.contains(&("conf/privilege".into(), 0o644)));

		let mut ar = tar::Archive::new(fs::File::open(&out).unwrap());
		let mut tgz = Vec::new();
		for e in ar.entries().unwrap() {
			let mut e = e.unwrap();
			if e.path().unwrap().to_str() == Some("package.tgz") {
				e.read_to_end(&mut tgz).unwrap();
			}
		}
		let mut inner = tar::Archive::new(flate2::read::GzDecoder::new(&tgz[..]));
		let inner: Vec<_> = inner
			.entries()
			.unwrap()
			.map(|e| {
				let e = e.unwrap();
				(
					e.path().unwrap().to_string_lossy().into_owned(),
					e.header().mode().unwrap(),
				)
			})
			.collect();
		assert_eq!(inner, vec![("bin/demo".to_string(), 0o755)]);
	}

	#[test]
	fn glob_needs_directory_dest() {
		let dir = tempfile::tempdir().unwrap();
		let manifest = crate::config::tests::write_manifest(
			dir.path(),
			"arch = [\"rtd1296\"]\nassets = [{ source = \"*.txt\", dest = \"x\" }]\n",
		);
		let config = Config::new(&manifest).unwrap();
		let target = BuildTarget::new(&Cli::default(), dir.path());
		assert!(expand_asset(&config, &target, &config.assets[0]).is_err());
	}
}
