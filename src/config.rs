use crate::error::Error;
use cargo_toml::{Manifest, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const METADATA_KEY: &str = "generate-spk";
const DEFAULT_OS_MIN_VER: &str = "7.0-40000";

#[derive(Debug, PartialEq)]
pub struct Asset {
	pub source: String,
	pub dest: String,
	pub mode: u32,
}

#[derive(Debug)]
pub struct Config {
	pub project_dir: PathBuf,
	pub name: String,
	pub version: String,
	pub description: String,
	pub displayname: String,
	pub maintainer: String,
	pub arch: Vec<String>,
	pub os_min_ver: String,
	pub startable: bool,
	pub scripts: Option<PathBuf>,
	pub conf: Option<PathBuf>,
	pub icon: Option<PathBuf>,
	pub icon_256: Option<PathBuf>,
	pub assets: Vec<Asset>,
	/// Extra `INFO` fields passed through as they are.
	pub info: BTreeMap<String, String>,
}

fn str_field(table: &Value, key: &str) -> Result<Option<String>, Error> {
	match table.get(key) {
		None => Ok(None),
		Some(Value::String(s)) => Ok(Some(s.clone())),
		Some(_) => Err(Error::Config(format!(
			"{METADATA_KEY}.{key} must be a string"
		))),
	}
}

fn path_field(table: &Value, key: &str) -> Result<Option<PathBuf>, Error> {
	Ok(str_field(table, key)?.map(PathBuf::from))
}

fn parse_mode(s: &str) -> Result<u32, Error> {
	u32::from_str_radix(s, 8)
		.map_err(|_| Error::Config(format!("invalid mode {s:?}, expected octal like \"0755\"")))
}

fn parse_assets(table: &Value) -> Result<Vec<Asset>, Error> {
	let items = match table.get("assets") {
		Some(Value::Array(a)) => a,
		_ => {
			return Err(Error::Config(format!(
				"{METADATA_KEY}.assets is mandatory and must be an array"
			)));
		}
	};
	items
		.iter()
		.map(|item| {
			let source = str_field(item, "source")?
				.ok_or_else(|| Error::Config("asset without `source`".into()))?;
			let dest = str_field(item, "dest")?
				.ok_or_else(|| Error::Config(format!("asset {source:?} without `dest`")))?;
			let mode = str_field(item, "mode")?.map_or(Ok(0o644), |m| parse_mode(&m))?;
			Ok(Asset { source, dest, mode })
		})
		.collect()
}

/// `Name <mail>` -> `Name`
fn author_name(author: &str) -> &str {
	author.split('<').next().unwrap_or(author).trim()
}

impl Config {
	pub fn new(manifest_path: &Path) -> Result<Config, Error> {
		let manifest_path = &manifest_path
			.canonicalize()
			.map_err(|e| Error::FileIo(manifest_path.to_path_buf(), e))?;
		let manifest = Manifest::<Value>::from_path(manifest_path)?;
		let package = manifest
			.package
			.as_ref()
			.ok_or_else(|| Error::Config("Cargo.toml has no [package]".into()))?;
		let meta = package
			.metadata
			.as_ref()
			.and_then(|m| m.get(METADATA_KEY))
			.ok_or_else(|| {
				Error::Config(format!(
					"missing [package.metadata.{METADATA_KEY}] in Cargo.toml"
				))
			})?;

		let name = str_field(meta, "name")?.unwrap_or_else(|| package.name().to_string());
		let version = str_field(meta, "version")?.unwrap_or_else(|| package.version().to_string());
		let description = str_field(meta, "description")?
			.or_else(|| package.description().map(String::from))
			.ok_or_else(|| Error::Config("missing description (package.description)".into()))?;
		let maintainer = str_field(meta, "maintainer")?
			.or_else(|| {
				package
					.authors()
					.first()
					.map(|a| author_name(a).to_string())
			})
			.ok_or_else(|| Error::Config("missing maintainer (package.authors)".into()))?;
		let arch: Vec<String> = match meta.get("arch") {
			Some(Value::Array(a)) => a
				.iter()
				.filter_map(|v| v.as_str().map(String::from))
				.collect(),
			Some(Value::String(s)) => s.split_whitespace().map(String::from).collect(),
			_ => vec![],
		};
		if arch.is_empty() {
			return Err(Error::Config(format!(
				"{METADATA_KEY}.arch is mandatory, e.g. [\"rtd1296\"]"
			)));
		}
		let startable = match meta.get("startable") {
			None => false,
			Some(Value::Boolean(b)) => *b,
			Some(_) => {
				return Err(Error::Config(format!(
					"{METADATA_KEY}.startable must be a boolean"
				)));
			}
		};
		let mut info = BTreeMap::new();
		if let Some(Value::Table(t)) = meta.get("info") {
			for (k, v) in t {
				let v = v
					.as_str()
					.ok_or_else(|| Error::Config(format!("info.{k} must be a string")))?;
				info.insert(k.clone(), v.to_string());
			}
		}

		let project_dir = match manifest_path.parent() {
			Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
			_ => PathBuf::from("."),
		};
		Ok(Config {
			project_dir,
			displayname: str_field(meta, "displayname")?.unwrap_or_else(|| name.clone()),
			os_min_ver: str_field(meta, "os_min_ver")?.unwrap_or_else(|| DEFAULT_OS_MIN_VER.into()),
			scripts: path_field(meta, "scripts")?,
			conf: path_field(meta, "conf")?,
			icon: path_field(meta, "icon")?,
			icon_256: path_field(meta, "icon_256")?,
			assets: parse_assets(meta)?,
			name,
			version,
			description,
			maintainer,
			arch,
			startable,
			info,
		})
	}

	/// File name of the package, e.g. `synodemo-0.1.0-rtd1296.spk`.
	pub fn file_name(&self) -> String {
		format!("{}-{}-{}.spk", self.name, self.version, self.arch[0])
	}

	/// Content of the `INFO` file.
	pub fn info_file(&self) -> String {
		let mut fields: Vec<(&str, String)> = vec![
			("package", self.name.clone()),
			("version", self.version.clone()),
			("os_min_ver", self.os_min_ver.clone()),
			("arch", self.arch.join(" ")),
			("maintainer", self.maintainer.clone()),
			("displayname", self.displayname.clone()),
			("description", self.description.clone()),
			(
				"startable",
				if self.startable { "yes" } else { "no" }.into(),
			),
		];
		fields.extend(self.info.iter().map(|(k, v)| (k.as_str(), v.clone())));
		fields
			.iter()
			.map(|(k, v)| format!("{k}=\"{}\"\n", v.replace('\\', "\\\\").replace('"', "\\\"")))
			.collect()
	}
}

#[cfg(test)]
pub(crate) mod tests {
	use super::*;

	pub fn write_manifest(dir: &Path, metadata: &str) -> PathBuf {
		std::fs::create_dir_all(dir.join("src")).unwrap();
		std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
		let path = dir.join("Cargo.toml");
		std::fs::write(
			&path,
			format!(
				"[package]\nname = \"demo\"\nversion = \"1.2.3\"\nedition = \"2024\"\n\
				 description = \"A demo\"\nauthors = [\"Jan Novák <jan@example.com>\"]\n\
				 [package.metadata.generate-spk]\n{metadata}"
			),
		)
		.unwrap();
		path
	}

	#[test]
	fn defaults_come_from_package() {
		let dir = tempfile::tempdir().unwrap();
		let path = write_manifest(
			dir.path(),
			"arch = [\"rtd1296\"]\nassets = [{ source = \"a\", dest = \"bin/a\", mode = \"0755\" }]\n",
		);
		let c = Config::new(&path).unwrap();
		assert_eq!((c.name.as_str(), c.version.as_str()), ("demo", "1.2.3"));
		assert_eq!(c.maintainer, "Jan Novák");
		assert_eq!(c.os_min_ver, DEFAULT_OS_MIN_VER);
		assert_eq!(
			c.assets,
			vec![Asset {
				source: "a".into(),
				dest: "bin/a".into(),
				mode: 0o755
			}]
		);
		assert_eq!(c.file_name(), "demo-1.2.3-rtd1296.spk");
	}

	#[test]
	fn info_file_escapes_quotes() {
		let dir = tempfile::tempdir().unwrap();
		let path = write_manifest(
			dir.path(),
			"arch = [\"rtd1296\", \"armada37xx\"]\nstartable = true\nassets = []\n\
			 [package.metadata.generate-spk.info]\nreport_url = \"http://x/\\\"y\"\n",
		);
		let info = Config::new(&path).unwrap().info_file();
		assert!(info.contains("arch=\"rtd1296 armada37xx\"\n"));
		assert!(info.contains("startable=\"yes\"\n"));
		assert!(info.contains("report_url=\"http://x/\\\"y\"\n"));
	}

	#[test]
	fn arch_is_mandatory() {
		let dir = tempfile::tempdir().unwrap();
		let path = write_manifest(dir.path(), "assets = []\n");
		assert!(matches!(Config::new(&path), Err(Error::Config(m)) if m.contains("arch")));
	}
}
