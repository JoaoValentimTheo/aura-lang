#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Aura 0.3 Keystone HTTP + typed-data dogfood — the PokéAPI chain.
//!
//! Authority: `docs/engineering/HTTP_ARCHITECTURE.md` (HTTP capability) and
//! `docs/LANGUAGE_SPEC.md` (typed JSON decode, `E4031`).
//!
//! The dogfood exercises the chain the Keystone scope names:
//!
//! ```text
//! Aura program → HTTP API → Host::http_request → capability check
//!   → HTTP response → JSON → json_decode / json_decode_as → Struct/optional
//! ```
//!
//! Deterministic CI runs the *real* transport and the *real* decoders against
//! a loopback server serving a captured PokéAPI document. That proves the
//! Aura-side chain without depending on a third-party service being reachable.
//! The live-network run is a separate opt-in test (`AURA_LIVE_POKEAPI=1`) so
//! CI never fails because PokéAPI is down.
//!
//! ## Finding: reserved-word keys in a real API
//!
//! PokéAPI's `types[].type` key is the Aura reserved word `type`
//! (`LANGUAGE_SPEC.md` §7), so a struct cannot declare a field with that name,
//! and `json_decode_as` requires the JSON key to equal the declared field name
//! exactly (it rejects both a missing field and an undeclared one). The typed
//! decoder therefore cannot consume a nested object whose key is reserved.
//! The dogfood proves the canonical response: the **dynamic** decoder
//! (`json_decode`, permissive, map access) reads the reserved-key part, and
//! the **typed** decoder reads the identifier-safe parts with exact
//! declarations. A field-rename mechanism would be new syntax and is recorded
//! as a human gate, not invented here.
#![cfg(all(feature = "http", feature = "json"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use aura::host::StdHost;
use aura::run::Interp;

/// A capture of `GET https://pokeapi.co/api/v2/pokemon/25` (Pikachu), trimmed
/// to the fields the dogfood reads, with the exact JSON shape of the live
/// endpoint (nested objects, arrays, and an optional field).
const PIKACHU_JSON: &str = r#"{
  "id": 25,
  "name": "pikachu",
  "height": 4,
  "weight": 60,
  "base_experience": 112,
  "is_default": true,
  "location_area_encounters": "https://pokeapi.co/api/v2/pokemon/25/encounters",
  "types": [
    { "slot": 1, "type": { "name": "electric", "url": "https://pokeapi.co/api/v2/type/13/" } }
  ],
  "stats": [
    { "base_stat": 35, "effort": 0, "stat": { "name": "hp", "url": "https://pokeapi.co/api/v2/stat/1/" } },
    { "base_stat": 55, "effort": 0, "stat": { "name": "attack", "url": "https://pokeapi.co/api/v2/stat/2/" } },
    { "base_stat": 90, "effort": 2, "stat": { "name": "speed", "url": "https://pokeapi.co/api/v2/stat/6/" } }
  ],
  "sprites": { "front_default": "https://raw.githubusercontent.com/PokeAPI/sprites/master/pokemon/25.png" },
  "held_items": []
}"#;

/// A capture of `GET https://pokeapi.co/api/v2/pokemon/99999` — the documented
/// 404 shape. The dogfood proves the failure class stays a plain HTTP status,
/// not an Aura exception.
const NOT_FOUND_BODY: &str = r#"{"detail":"Not found."}"#;

struct TestServer {
    port: u16,
    stop: Arc<AtomicBool>,
}

impl TestServer {
    fn start(response: String) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().expect("addr").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if stop_clone.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = stream.write_all(response.as_bytes());
            }
        });
        TestServer { port, stop }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

fn json_response(body: &str, status: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[derive(Clone)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run(src: &str) -> Result<String, u16> {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let host = StdHost::from_parts(Box::new(Sink(Arc::clone(&buf))), Vec::new(), None);
    let mut it = Interp::with_host(Box::new(host));
    aura::stdlib::install(&mut it);
    let module = aura::parse::parse(src).expect("parses");
    aura::check::Checker::module(&module).map_err(|d| d.code)?;
    it.run(&module).map_err(|d| d.code)?;
    let bytes = buf.lock().unwrap().clone();
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// The declared types the dogfood uses for the identifier-safe parts of the
/// response. Every field is declared exactly (typed decode rejects undeclared
/// fields), and `front_default` is optional.
const DECLARATIONS: &str = r#"
struct Named { name: string, url: string }
struct Stat { base_stat: int, effort: int, stat: Named }
struct SpriteRef { back_default: string | none, front_default: string | none }
"#;

/// The dogfood program: fetch, decode dynamically for the reserved-key part,
/// and decode typed for the identifier-safe nested parts.
fn dogfood_program(base: &str) -> String {
    format!(
        r#"{DECLARATIONS}
fn main() {{
    let r = http_get("{base}/api/v2/pokemon/25")
    print(r["status"])
    let p = json_decode(r["body"])
    print(p["name"])
    print(p["id"])
    print(p["types"][0]["type"]["name"])
    let stats = json_decode_as(json_encode(p["stats"]), "[Stat]")
    print(stats[0].base_stat)
    print(stats[2].stat.name)
    print(len(stats))
    let sprites = json_decode_as(json_encode(p["sprites"]), "SpriteRef")
    print(sprites.front_default != none)
    print(p["is_default"])
}}"#
    )
}

#[test]
fn pokeapi_chain_fetches_decodes_and_reads_nested_and_typed_data() {
    let server = TestServer::start(json_response(PIKACHU_JSON, "200 OK"));
    let out = run(&dogfood_program(&server.url(""))).expect("dogfood runs");
    assert_eq!(
        out,
        "200\npikachu\n25\nelectric\n35\nspeed\n3\ntrue\ntrue\n"
    );
}

#[test]
fn pokeapi_chain_accepts_an_absent_optional_field_as_none() {
    // `front_default` is optional (`string | none`). A response that omits it
    // must decode to `none`, not fail.
    let body = PIKACHU_JSON.replace(
        r#""sprites": { "front_default": "https://raw.githubusercontent.com/PokeAPI/sprites/master/pokemon/25.png" },"#,
        r#""sprites": {},"#,
    );
    assert!(!body.contains("front_default"), "fixture was trimmed");
    let server = TestServer::start(json_response(&body, "200 OK"));
    let src = dogfood_program(&server.url("")).replace(
        "print(sprites.front_default != none)",
        "print(sprites.front_default == none)",
    );
    let out = run(&src).expect("dogfood runs without the optional field");
    assert_eq!(
        out,
        "200\npikachu\n25\nelectric\n35\nspeed\n3\ntrue\ntrue\n"
    );
}

#[test]
fn pokeapi_chain_keeps_a_404_a_plain_status_with_a_decodable_body() {
    // The live API answers an unknown id with 404 and a JSON `detail`. The
    // failure is a status the program decides on, never an Aura exception and
    // never a silent success.
    let server = TestServer::start(json_response(NOT_FOUND_BODY, "404 Not Found"));
    let src = format!(
        r#"struct Detail {{ detail: string }}
fn main() {{
    let r = http_get("{}/api/v2/pokemon/99999")
    print(r["status"])
    let d = json_decode_as(r["body"], "Detail")
    print(d.detail)
}}"#,
        server.url("")
    );
    let out = run(&src).expect("the 404 is an ordinary response");
    assert_eq!(out, "404\nNot found.\n");
}

#[test]
fn pokeapi_chain_decode_mismatch_reports_the_structured_code() {
    // A document whose nested shape does not match the declared type is
    // `E4031` (decode mismatch), naming the failure precisely.
    let body = PIKACHU_JSON.replace("\"base_stat\": 35", "\"base_stat\": \"thirty-five\"");
    let server = TestServer::start(json_response(&body, "200 OK"));
    let src = format!(
        r#"{DECLARATIONS}
fn main() {{
    let r = http_get("{}/api/v2/pokemon/25")
    let p = json_decode(r["body"])
    let _stats = json_decode_as(json_encode(p["stats"]), "[Stat]")
}}"#,
        server.url("")
    );
    let code = run(&src).expect_err("the type mismatch is rejected");
    assert_eq!(code, aura::error::codes::DECODE_MISMATCH);
}

#[test]
fn pokeapi_chain_decode_mismatch_is_not_a_catchable_user_exception() {
    // Level separation (§14.5): an `E4031` decode mismatch is an ordinary
    // diagnostic. A `try` around it does not intercept it — the program fails
    // with the diagnostic rather than printing "caught".
    let body = PIKACHU_JSON.replace("\"base_stat\": 35", "\"base_stat\": \"thirty-five\"");
    let server = TestServer::start(json_response(&body, "200 OK"));
    let src = format!(
        r#"{DECLARATIONS}
fn main() {{
    try {{
        let r = http_get("{}/api/v2/pokemon/25")
        let p = json_decode(r["body"])
        let _stats = json_decode_as(json_encode(p["stats"]), "[Stat]")
        print("decoded")
    }} catch _ {{
        print("caught")
    }}
}}"#,
        server.url("")
    );
    let code = run(&src).expect_err("the mismatch escapes the try");
    assert_eq!(code, aura::error::codes::DECODE_MISMATCH);
}

#[test]
fn pokeapi_chain_rejects_an_undeclared_field_rather_than_ignoring_it() {
    // Typed decode is strict: an extra JSON field is a mismatch, so a
    // contractual change in the API is surfaced instead of silently dropped.
    let server = TestServer::start(json_response(PIKACHU_JSON, "200 OK"));
    let src = format!(
        r#"struct TooSmall {{ name: string }}
fn main() {{
    let r = http_get("{}/api/v2/pokemon/25")
    let _ = json_decode_as(r["body"], "TooSmall")
}}"#,
        server.url("")
    );
    let code = run(&src).expect_err("undeclared fields are rejected");
    assert_eq!(code, aura::error::codes::DECODE_MISMATCH);
}

/// The live-network dogfood. Opt-in so CI never depends on a third party:
///
/// ```text
/// AURA_LIVE_POKEAPI=1 cargo test --locked --all-features \
///     --test pokeapi_dogfood -- --ignored
/// ```
///
/// It runs the same program against the real endpoint through the real
/// capability, transport, and decoders, so the fixture path is proven to match
/// the live shape rather than a shape we invented.
#[test]
#[ignore = "live network: opt in with AURA_LIVE_POKEAPI=1 and --ignored"]
fn live_pokeapi_chain_decodes_pikachu() {
    if std::env::var("AURA_LIVE_POKEAPI").as_deref() != Ok("1") {
        eprintln!("skipping live dogfood: set AURA_LIVE_POKEAPI=1 to run");
        return;
    }
    let out = run(&dogfood_program("https://pokeapi.co")).expect("live dogfood runs");
    assert_eq!(
        out,
        "200\npikachu\n25\nelectric\n35\nspeed\n3\ntrue\ntrue\n"
    );
}
