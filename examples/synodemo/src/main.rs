use std::io::{Read, Write};
use std::net::TcpListener;

fn main() -> std::io::Result<()> {
	let listener = TcpListener::bind("0.0.0.0:8085")?;
	eprintln!("Listening on :8085");
	for stream in listener.incoming() {
		let mut s = stream?;
		let mut buf = [0u8; 1024];
		let _ = s.read(&mut buf);
		let body = "Ahoj z Rustu na Synology!\n";
		write!(
			s,
			"HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
			body.len(),
			body
		)?;
	}
	Ok(())
}
