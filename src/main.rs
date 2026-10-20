use build_target::BuildTarget;
use cli::Cli;
use config::Config;
use error::Error;
use std::path::PathBuf;

mod build_target;
mod cli;
mod config;
mod error;
mod spk;

fn run() -> Result<(), Error> {
	let args = Cli::parse_args(std::env::args_os());
	let manifest_path = args
		.manifest_path
		.clone()
		.unwrap_or_else(|| PathBuf::from("Cargo.toml"));
	let config = Config::new(&manifest_path)?;
	let target = BuildTarget::new(&args, &config.project_dir);

	let file_name = config.file_name();
	let output = match &args.output {
		Some(p) if p.is_dir() => p.join(&file_name),
		Some(p) => p.clone(),
		None => target.output_dir().join(&file_name),
	};
	spk::build(&config, &target, &output)?;
	println!("{}", output.display());
	Ok(())
}

fn main() {
	if let Err(err) = run() {
		eprintln!("error: {err}");
		std::process::exit(1);
	}
}
