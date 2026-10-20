use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Parser, Debug, Default)]
#[command(
	name = "cargo-generate-spk",
	bin_name = "cargo generate-spk",
	version,
	about
)]
pub struct Cli {
	/// Path to Cargo.toml
	#[arg(long, value_name = "PATH")]
	pub manifest_path: Option<PathBuf>,

	/// Build target triple the binaries were built for (e.g. aarch64-unknown-linux-musl)
	#[arg(long)]
	pub target: Option<String>,

	/// Cargo profile the binaries were built with
	#[arg(long, default_value = "release")]
	pub profile: String,

	/// Output file or directory (default: target/generate-spk/)
	#[arg(short, long)]
	pub output: Option<PathBuf>,
}

impl Cli {
	/// Cargo invokes external subcommands as `cargo-generate-spk generate-spk ...`.
	pub fn parse_args(args: impl IntoIterator<Item = OsString>) -> Cli {
		let mut args: Vec<OsString> = args.into_iter().collect();
		if args.get(1).is_some_and(|a| a == "generate-spk") {
			args.remove(1);
		}
		Cli::parse_from(args)
	}
}
