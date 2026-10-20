use crate::cli::Cli;
use std::path::{Path, PathBuf};

/// Where cargo put the build artifacts.
pub struct BuildTarget {
	target: Option<String>,
	profile: String,
	base: PathBuf,
}

impl BuildTarget {
	pub fn new(args: &Cli, project_dir: &Path) -> Self {
		let base = std::env::var_os("CARGO_TARGET_DIR")
			.map(PathBuf::from)
			.unwrap_or_else(|| project_dir.join("target"));
		BuildTarget {
			target: args.target.clone(),
			profile: args.profile.clone(),
			base,
		}
	}

	/// Directory with built binaries, e.g. `target/aarch64-unknown-linux-musl/release`.
	pub fn artifact_dir(&self) -> PathBuf {
		let profile_dir = if self.profile == "dev" {
			"debug"
		} else {
			&self.profile
		};
		match &self.target {
			Some(t) => self.base.join(t).join(profile_dir),
			None => self.base.join(profile_dir),
		}
	}

	pub fn output_dir(&self) -> PathBuf {
		self.base.join("generate-spk")
	}

	/// `target/release/x` in an asset source means the artifact directory of this build.
	pub fn resolve(&self, project_dir: &Path, source: &str) -> PathBuf {
		match source.strip_prefix("target/release/") {
			Some(rest) => self.artifact_dir().join(rest),
			None => project_dir.join(source),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn target(triple: Option<&str>, profile: &str) -> BuildTarget {
		BuildTarget {
			target: triple.map(String::from),
			profile: profile.into(),
			base: PathBuf::from("/p/target"),
		}
	}

	#[test]
	fn resolves_release_prefix() {
		let t = target(Some("aarch64-unknown-linux-musl"), "release");
		assert_eq!(
			t.resolve(Path::new("/p"), "target/release/x"),
			PathBuf::from("/p/target/aarch64-unknown-linux-musl/release/x")
		);
		assert_eq!(
			t.resolve(Path::new("/p"), "README.md"),
			PathBuf::from("/p/README.md")
		);
	}

	#[test]
	fn dev_profile_is_debug_dir() {
		assert_eq!(
			target(None, "dev").artifact_dir(),
			PathBuf::from("/p/target/debug")
		);
	}
}
