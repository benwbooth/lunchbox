use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::{Context, Result};
use flate2::Compression;
use flate2::write::ZlibEncoder;

fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").context("binding IGDB fixture server")?;
    println!(
        "LUNCHBOX_IGDB_FIXTURE_READY http://{}",
        listener.local_addr()?
    );
    std::io::stdout().flush()?;
    let image = fixture_png()?;
    for stream in listener.incoming() {
        let mut stream = stream.context("accepting IGDB fixture request")?;
        if let Err(error) = serve(&mut stream, &image) {
            eprintln!("LUNCHBOX_IGDB_FIXTURE_REQUEST_FAILED {error:#}");
        }
    }
    Ok(())
}

fn serve(stream: &mut TcpStream, image: &[u8]) -> Result<()> {
    let request = read_request(stream)?;
    let request_text = String::from_utf8_lossy(&request);
    let request_line = request_text.lines().next().unwrap_or_default();
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    eprintln!("LUNCHBOX_IGDB_FIXTURE_REQUEST {request_line}");
    match path {
        "/oauth2/token" => reply(
            stream,
            200,
            "application/json",
            br#"{"access_token":"fixture-token","expires_in":3600,"token_type":"bearer"}"#,
        ),
        "/v4/games" => reply(stream, 200, "application/json", fixture_game()),
        path if path.starts_with("/images/") => reply(stream, 200, "image/png", image),
        _ => reply(stream, 404, "text/plain", b"not found"),
    }
}

fn read_request(stream: &mut TcpStream) -> Result<Vec<u8>> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    let mut content_length = None;
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            let header_end = header_end + 4;
            if content_length.is_none() {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                content_length = headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                });
            }
            if bytes.len() >= header_end + content_length.unwrap_or(0) {
                break;
            }
        }
    }
    Ok(bytes)
}

fn reply(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) -> Result<()> {
    let reason = if status == 200 { "OK" } else { "Not Found" };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nContent-Type: {content_type}\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()?;
    Ok(())
}

fn fixture_game() -> &'static [u8] {
    br#"[{"id":7346,"name":"Super Mario Bros.","first_release_date":499132800,"platforms":[{"name":"Nintendo Entertainment System","abbreviation":"NES"}],"cover":{"id":11,"image_id":"co1abc","width":528,"height":748},"screenshots":[{"id":12,"image_id":"sc1abc","width":1280,"height":720}],"artworks":[{"id":13,"image_id":"ar1abc","width":1920,"height":1080}]}]"#
}

fn fixture_png() -> Result<Vec<u8>> {
    const WIDTH: u32 = 960;
    const HEIGHT: u32 = 540;
    let mut raw = Vec::with_capacity((WIDTH as usize * 4 + 1) * HEIGHT as usize);
    for y in 0..HEIGHT {
        raw.push(0);
        for x in 0..WIDTH {
            let band = ((x / 120) + (y / 90)) % 2;
            let glow = ((x * 90 / WIDTH) + (y * 70 / HEIGHT)) as u8;
            if band == 0 {
                raw.extend_from_slice(&[
                    26 + glow / 3,
                    63 + glow / 2,
                    105_u8.saturating_add(glow),
                    255,
                ]);
            } else {
                raw.extend_from_slice(&[
                    92_u8.saturating_add(glow),
                    39 + glow / 3,
                    72 + glow / 2,
                    255,
                ]);
            }
        }
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&raw)?;
    let compressed = encoder.finish()?;

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&WIDTH.to_be_bytes());
    header.extend_from_slice(&HEIGHT.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    push_chunk(&mut png, b"IHDR", &header);
    push_chunk(&mut png, b"IDAT", &compressed);
    push_chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

fn push_chunk(png: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
    png.extend_from_slice(&(body.len() as u32).to_be_bytes());
    png.extend_from_slice(kind);
    png.extend_from_slice(body);
    let mut crc = crc32fast::Hasher::new();
    crc.update(kind);
    crc.update(body);
    png.extend_from_slice(&crc.finalize().to_be_bytes());
}
