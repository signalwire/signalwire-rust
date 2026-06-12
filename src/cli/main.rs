use std::collections::HashMap;
use std::env;
use std::process;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{json, Value};

/// CLI entry point for the `swaig-test` tool.
///
/// Usage:
///   swaig-test --example <NAME> --list-tools
///   swaig-test --url <URL> [options]
///
/// Options:
///   --example <NAME>     SWMLService example to introspect by name (e.g.
///                        `swmlservice_swaig_standalone`). Runs the example
///                        in-process via `cargo run --example` with the
///                        `SWAIG_LIST_TOOLS=1` env var; the SDK's serve()
///                        dumps the runtime registry instead of binding a
///                        port.
///   --url <URL>          SWAIG endpoint URL. Basic auth can be embedded as
///                        user:pass@host.
///   --dump-swml          Fetch and dump the SWML document (URL mode).
///   --list-tools         List available SWAIG tools.
///   --exec <TOOL>        Execute a specific SWAIG tool by name (URL mode).
///   --param <K=V>        Parameter for --exec (repeatable).
///   --raw                Print raw JSON responses (no formatting).
///   --verbose            Enable verbose output.
///   --help               Print this help message.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || args.contains(&"--help".to_string()) {
        print_help();
        process::exit(0);
    }

    let mut url: Option<String> = None;
    let mut example: Option<String> = None;
    let mut dump_swml = false;
    let mut list_tools = false;
    let mut exec_tool: Option<String> = None;
    let mut params: Vec<(String, String)> = Vec::new();
    let mut raw = false;
    let mut verbose = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--url" => {
                i += 1;
                if i < args.len() {
                    url = Some(args[i].clone());
                } else {
                    eprintln!("Error: --url requires a value");
                    process::exit(1);
                }
            }
            "--example" => {
                i += 1;
                if i < args.len() {
                    example = Some(args[i].clone());
                } else {
                    eprintln!("Error: --example requires a name");
                    process::exit(1);
                }
            }
            "--dump-swml" => dump_swml = true,
            "--list-tools" => list_tools = true,
            "--exec" => {
                i += 1;
                if i < args.len() {
                    exec_tool = Some(args[i].clone());
                } else {
                    eprintln!("Error: --exec requires a tool name");
                    process::exit(1);
                }
            }
            "--param" => {
                i += 1;
                if i < args.len() {
                    if let Some(eq_pos) = args[i].find('=') {
                        let k = args[i][..eq_pos].to_string();
                        let v = args[i][eq_pos + 1..].to_string();
                        params.push((k, v));
                    } else {
                        eprintln!("Error: --param requires K=V format");
                        process::exit(1);
                    }
                } else {
                    eprintln!("Error: --param requires a value");
                    process::exit(1);
                }
            }
            "--raw" => raw = true,
            "--verbose" => verbose = true,
            "--help" => {
                print_help();
                process::exit(0);
            }
            other => {
                eprintln!("Error: unknown option: {other}");
                process::exit(1);
            }
        }
        i += 1;
    }

    // Example/file-loader mode.
    if let Some(name) = example {
        if !list_tools {
            eprintln!("Error: --example currently only supports --list-tools");
            process::exit(1);
        }
        do_list_tools_via_introspect(&name, raw, verbose);
        return;
    }

    let url = if let Some(u) = url { u } else {
        eprintln!("Error: --url or --example is required");
        process::exit(1);
    };

    // Extract auth from URL if embedded
    let (base_url, auth_header) = extract_url_auth(&url);

    if verbose {
        eprintln!("[verbose] URL: {base_url}");
        if auth_header.is_some() {
            eprintln!("[verbose] Auth: (embedded credentials)");
        }
    }

    // Route to the appropriate action
    if dump_swml {
        do_dump_swml(&base_url, &auth_header, raw, verbose);
    } else if list_tools {
        do_list_tools(&base_url, &auth_header, raw, verbose);
    } else if let Some(tool) = exec_tool {
        do_exec_tool(&base_url, &auth_header, &tool, &params, raw, verbose);
    } else {
        eprintln!("Error: specify --dump-swml, --list-tools, or --exec <tool>");
        process::exit(1);
    }
}

fn print_help() {
    println!("swaig-test - SignalWire SWAIG testing tool");
    println!();
    println!("Usage:");
    println!("  swaig-test --example <NAME> --list-tools");
    println!("  swaig-test --url <URL> [options]");
    println!();
    println!("Options:");
    println!("  --example <NAME> Introspect a built example by cargo example name.");
    println!("                   Runs `cargo run --example <NAME>` with");
    println!("                   SWAIG_LIST_TOOLS=1 set; the SDK's serve() dumps");
    println!("                   the runtime tool registry and exits without");
    println!("                   binding any port.");
    println!("  --url <URL>      SWAIG endpoint URL (HTTP mode)");
    println!("  --dump-swml      Fetch and dump the SWML document (URL mode)");
    println!("  --list-tools     List available SWAIG tools");
    println!("  --exec <TOOL>    Execute a specific SWAIG tool (URL mode)");
    println!("  --param <K=V>    Parameter for --exec (repeatable)");
    println!("  --raw            Print raw JSON (no formatting)");
    println!("  --verbose        Enable verbose output");
    println!("  --help           Print this help message");
    println!();
    println!("Auth:");
    println!("  Embed credentials in the URL: http://user:pass@host:port/path");
}

/// Introspect a SWMLService example by spawning `cargo run --example NAME`
/// with `SWAIG_LIST_TOOLS=1`. The SDK's `Service::run()` honors that env var
/// by printing the registry to stdout between sentinels and exiting before
/// it would have bound any port. We capture stdout, slice out the JSON, and
/// pretty-print or emit raw.
fn do_list_tools_via_introspect(example_name: &str, raw: bool, verbose: bool) {
    if verbose {
        eprintln!("[verbose] running `cargo run --example {example_name}` with SWAIG_LIST_TOOLS=1");
    }
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["run", "--quiet", "--example", example_name])
        .env("SWAIG_LIST_TOOLS", "1");
    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Error: failed to spawn cargo: {e}");
            process::exit(1);
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Error: example `{example_name}` exited non-zero");
        if !stderr.is_empty() {
            eprintln!("--- cargo stderr ---\n{}", stderr.trim_end());
        }
        process::exit(1);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let body = if let Some(s) = extract_introspect_payload(&stdout) { s } else {
        eprintln!(
            "Error: example `{example_name}` did not emit __SWAIG_TOOLS_BEGIN__/__SWAIG_TOOLS_END__ markers. Make sure it calls service.run()."
        );
        if verbose {
            eprintln!("--- raw stdout ---\n{stdout}");
        }
        process::exit(1);
    };
    let parsed: Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: malformed introspect payload: {e}");
            eprintln!("--- payload ---\n{body}");
            process::exit(1);
        }
    };
    if raw {
        println!("{body}");
        return;
    }
    let tools = parsed.get("tools").and_then(|v| v.as_array());
    let tools = if let Some(a) = tools { a } else {
        println!("{}", serde_json::to_string_pretty(&parsed).unwrap_or_default());
        return;
    };
    if tools.is_empty() {
        println!("No tools registered.");
        return;
    }
    println!("Registered SWAIG tools ({}):", tools.len());
    for (i, tool) in tools.iter().enumerate() {
        let name = tool
            .get("function")
            .and_then(|v| v.as_str())
            .or_else(|| tool.get("name").and_then(|v| v.as_str()))
            .unwrap_or("<unnamed>");
        let desc = tool
            .get("purpose")
            .and_then(|v| v.as_str())
            .or_else(|| tool.get("description").and_then(|v| v.as_str()))
            .unwrap_or("");
        println!("  {}. {} — {}", i + 1, name, desc);
        let argument = tool.get("argument").or_else(|| tool.get("parameters"));
        if let Some(arg) = argument
            && let Some(props) = arg.get("properties").and_then(|v| v.as_object()) {
                for (pname, pdef) in props {
                    let ptype = pdef.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    let pdesc = pdef.get("description").and_then(|v| v.as_str()).unwrap_or("");
                    println!("       - {pname} ({ptype}): {pdesc}");
                }
            }
    }
}

/// Extract the JSON payload between __SWAIG_TOOLS_BEGIN__ and
/// __SWAIG_TOOLS_END__ markers in the example's stdout. Returns None if
/// either marker is missing.
fn extract_introspect_payload(stdout: &str) -> Option<&str> {
    let begin = stdout.find("__SWAIG_TOOLS_BEGIN__")?;
    let after_begin = &stdout[begin + "__SWAIG_TOOLS_BEGIN__".len()..];
    let end = after_begin.find("__SWAIG_TOOLS_END__")?;
    Some(after_begin[..end].trim())
}

/// Extract Basic auth credentials from a URL of the form
/// `http://user:pass@host:port/path` and return the cleaned URL + auth header.
fn extract_url_auth(url: &str) -> (String, Option<String>) {
    // Find the scheme separator
    let scheme_end = match url.find("://") {
        Some(pos) => pos + 3,
        None => return (url.to_string(), None),
    };

    let scheme = &url[..scheme_end];
    let rest = &url[scheme_end..];

    // Look for @ in the authority section (before path)
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let path_and_rest = &rest[authority_end..];

    if let Some(at_pos) = authority.find('@') {
        let user_pass = &authority[..at_pos];
        let host = &authority[at_pos + 1..];

        let auth = format!("Basic {}", BASE64.encode(user_pass));
        let clean_url = format!("{scheme}{host}{path_and_rest}");

        (clean_url, Some(auth))
    } else {
        (url.to_string(), None)
    }
}

/// Build request headers with optional auth.
fn build_headers(auth: &Option<String>) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    if let Some(a) = auth {
        headers.insert("Authorization".to_string(), a.clone());
    }
    headers
}

/// Make an HTTP request via `ureq`.
///
/// 4xx / 5xx are not transport failures: the agent returns the status code
/// and body verbatim and lets the caller decide how to react. The only
/// `Err` cases are genuine I/O / DNS / TLS / parse errors that prevented
/// us from getting a status line back.
///
/// 30-second global timeout. Verbose mode mirrors method/URL/headers/body
/// to stderr before the call.
fn http_request(
    method: &str,
    url: &str,
    headers: &HashMap<String, String>,
    body: Option<&str>,
    verbose: bool,
) -> Result<(u16, String), String> {
    if verbose {
        eprintln!("[verbose] {method} {url}");
        for (k, v) in headers {
            eprintln!("[verbose]   {k}: {v}");
        }
        if let Some(b) = body {
            eprintln!("[verbose]   body: {b}");
        }
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into();

    let response_result = match method.to_ascii_uppercase().as_str() {
        "GET" => {
            let mut req = agent.get(url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            req.call()
        }
        "POST" => {
            let mut req = agent.post(url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            match body {
                Some(b) => req.send(b),
                None => req.send_empty(),
            }
        }
        "PUT" => {
            let mut req = agent.put(url);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            match body {
                Some(b) => req.send(b),
                None => req.send_empty(),
            }
        }
        other => {
            return Err(format!("Unsupported HTTP method: {other}"));
        }
    };

    let mut response = response_result.map_err(|e| format!("HTTP {method} {url} failed: {e}"))?;
    let status = response.status().as_u16();
    let body_str = response
        .body_mut()
        .read_to_string()
        .map_err(|e| format!("HTTP {method} {url} body read failed: {e}"))?;
    Ok((status, body_str))
}

fn do_dump_swml(base_url: &str, auth: &Option<String>, raw: bool, verbose: bool) {
    let headers = build_headers(auth);
    match http_request("GET", base_url, &headers, None, verbose) {
        Ok((_status, body)) => {
            if raw {
                println!("{body}");
            } else {
                match serde_json::from_str::<Value>(&body) {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(_) => println!("{body}"),
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn do_list_tools(base_url: &str, auth: &Option<String>, raw: bool, verbose: bool) {
    let swaig_url = format!("{}/swaig", base_url.trim_end_matches('/'));
    let headers = build_headers(auth);
    match http_request("GET", &swaig_url, &headers, None, verbose) {
        Ok((_status, body)) => {
            if raw {
                println!("{body}");
            } else {
                match serde_json::from_str::<Value>(&body) {
                    Ok(v) => {
                        if let Some(arr) = v.as_array() {
                            if arr.is_empty() {
                                println!("No tools available.");
                            } else {
                                for (i, tool) in arr.iter().enumerate() {
                                    let name = tool
                                        .get("function")
                                        .and_then(|f| f.get("name"))
                                        .and_then(|n| n.as_str())
                                        .or_else(|| {
                                            tool.get("name").and_then(|n| n.as_str())
                                        })
                                        .unwrap_or("<unnamed>");
                                    let desc = tool
                                        .get("function")
                                        .and_then(|f| f.get("description"))
                                        .and_then(|d| d.as_str())
                                        .or_else(|| {
                                            tool.get("description").and_then(|d| d.as_str())
                                        })
                                        .unwrap_or("");
                                    println!("  {}. {} - {}", i + 1, name, desc);
                                }
                            }
                        } else {
                            println!("{}", serde_json::to_string_pretty(&v).unwrap());
                        }
                    }
                    Err(_) => println!("{body}"),
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

fn do_exec_tool(
    base_url: &str,
    auth: &Option<String>,
    tool: &str,
    params: &[(String, String)],
    raw: bool,
    verbose: bool,
) {
    let swaig_url = format!("{}/swaig", base_url.trim_end_matches('/'));
    let headers = build_headers(auth);

    // Build argument object
    let mut args = serde_json::Map::new();
    for (k, v) in params {
        // Try to parse as JSON first, fall back to string
        let val: Value = serde_json::from_str(v).unwrap_or(Value::String(v.clone()));
        args.insert(k.clone(), val);
    }

    let body = json!({
        "action": "execute",
        "function": tool,
        "argument": {
            "parsed": [args],
        },
    });

    let body_str = serde_json::to_string(&body).unwrap();

    match http_request("POST", &swaig_url, &headers, Some(&body_str), verbose) {
        Ok((_status, resp_body)) => {
            if raw {
                println!("{resp_body}");
            } else {
                match serde_json::from_str::<Value>(&resp_body) {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(_) => println!("{resp_body}"),
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ------------------------------------------------------------------
// Tests (library functions only -- main() is not tested here)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Captured inbound requests recorded by `spawn_test_server`: a shared
    /// `(method, path, headers, body)` log the test thread appends to.
    type CapturedRequests =
        std::sync::Arc<std::sync::Mutex<Vec<(String, String, HashMap<String, String>, String)>>>;

    #[test]
    fn test_extract_url_auth_with_creds() {
        let (url, auth) = extract_url_auth("http://user:pass@localhost:3000/api");
        assert_eq!(url, "http://localhost:3000/api");
        assert!(auth.is_some());
        let auth_val = auth.unwrap();
        assert!(auth_val.starts_with("Basic "));
        // Decode to verify
        let encoded = &auth_val[6..];
        let decoded = BASE64.decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "user:pass");
    }

    #[test]
    fn test_extract_url_auth_without_creds() {
        let (url, auth) = extract_url_auth("http://localhost:3000/api");
        assert_eq!(url, "http://localhost:3000/api");
        assert!(auth.is_none());
    }

    #[test]
    fn test_extract_url_auth_https() {
        let (url, auth) = extract_url_auth("https://admin:secret@api.example.com/v1");
        assert_eq!(url, "https://api.example.com/v1");
        assert!(auth.is_some());
    }

    #[test]
    fn test_extract_url_auth_no_path() {
        let (url, auth) = extract_url_auth("http://user:pass@localhost");
        assert_eq!(url, "http://localhost");
        assert!(auth.is_some());
    }

    #[test]
    fn test_extract_url_auth_no_scheme() {
        let (url, auth) = extract_url_auth("localhost:3000/api");
        assert_eq!(url, "localhost:3000/api");
        assert!(auth.is_none());
    }

    #[test]
    fn test_extract_url_auth_special_chars() {
        let (url, auth) = extract_url_auth("http://user%40:p%40ss@host/path");
        assert_eq!(url, "http://host/path");
        assert!(auth.is_some());
    }

    #[test]
    fn test_build_headers_with_auth() {
        let auth = Some("Basic dGVzdDp0ZXN0".to_string());
        let headers = build_headers(&auth);
        assert_eq!(headers["Authorization"], "Basic dGVzdDp0ZXN0");
        assert_eq!(headers["Content-Type"], "application/json");
        assert_eq!(headers["Accept"], "application/json");
    }

    #[test]
    fn test_build_headers_without_auth() {
        let headers = build_headers(&None);
        assert!(!headers.contains_key("Authorization"));
        assert_eq!(headers["Content-Type"], "application/json");
    }

    /// Spawn a tiny_http server on an ephemeral port that responds with the
    /// given fixed status + body, capturing whatever the request was. Returns
    /// (base_url, request_capture). Used by the http_request behavior tests
    /// below. Killed when the returned guard drops.
    fn spawn_test_server(
        status: u16,
        response_body: &'static str,
    ) -> (String, CapturedRequests, std::thread::JoinHandle<()>) {
        let server = tiny_http::Server::http("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = server.server_addr().to_ip().unwrap().port();
        let base = format!("http://127.0.0.1:{port}");
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let cap_clone = std::sync::Arc::clone(&captured);
        let handle = std::thread::spawn(move || {
            for mut req in server.incoming_requests() {
                let method = req.method().as_str().to_string();
                let path = req.url().to_string();
                let mut hmap = HashMap::new();
                for h in req.headers() {
                    hmap.insert(
                        h.field.as_str().as_str().to_string(),
                        h.value.as_str().to_string(),
                    );
                }
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                cap_clone.lock().unwrap().push((method, path, hmap, body));
                let resp = tiny_http::Response::from_string(response_body)
                    .with_status_code(status);
                let _ = req.respond(resp);
            }
        });
        (base, captured, handle)
    }

    #[test]
    fn test_http_request_get_round_trips_real_response() {
        // Real GET against a real local HTTP server. Was previously a stub
        // test asserting `err.contains("HTTP transport not available")` —
        // that test ratified the stub. Now the function does real I/O.
        let (base, captured, _h) = spawn_test_server(200, r#"{"ok":true,"verb":"GET"}"#);
        let url = format!("{base}/swml");
        let headers = build_headers(&None);
        let (status, body) = http_request("GET", &url, &headers, None, false)
            .expect("real GET should succeed against the test server");
        assert_eq!(status, 200);
        assert!(body.contains("\"ok\":true"));
        let cap = captured.lock().unwrap();
        assert_eq!(cap.len(), 1, "exactly one request hit the test server");
        assert_eq!(cap[0].0, "GET");
        assert_eq!(cap[0].1, "/swml");
    }

    #[test]
    fn test_http_request_post_forwards_body_and_basic_auth() {
        let (base, captured, _h) = spawn_test_server(
            200,
            r#"{"function":"lookup","response":"ACME"}"#,
        );
        let url = format!("{base}/swaig");
        let mut headers = build_headers(&Some("Basic dGVzdDp0ZXN0".to_string()));
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        let body = r#"{"function":"lookup","argument":{"parsed":[{"competitor":"ACME"}]}}"#;
        let (status, resp) = http_request("POST", &url, &headers, Some(body), false)
            .expect("real POST should succeed");
        assert_eq!(status, 200);
        assert!(resp.contains("\"response\":\"ACME\""));
        let cap = captured.lock().unwrap();
        assert_eq!(cap[0].0, "POST");
        assert_eq!(cap[0].1, "/swaig");
        // Auth header forwarded verbatim. Header names are case-insensitive
        // in HTTP — different stacks normalize differently — so look it up
        // case-insensitively.
        let auth = cap[0]
            .2
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Authorization"))
            .map(|(_, v)| v.as_str());
        assert_eq!(auth, Some("Basic dGVzdDp0ZXN0"));
        // Body forwarded verbatim.
        assert!(cap[0].3.contains("ACME"));
    }

    #[test]
    fn test_http_request_propagates_4xx_status_with_body() {
        // 4xx is NOT a transport failure — http_request returns (status, body)
        // and lets the caller decide how to react. Asserts that contract.
        let (base, _captured, _h) = spawn_test_server(404, r#"{"error":"not found"}"#);
        let (status, body) = http_request("GET", &format!("{base}/missing"), &HashMap::new(), None, false)
            .expect("4xx is not an Err — it's a status the caller will handle");
        assert_eq!(status, 404);
        assert!(body.contains("not found"));
    }

    #[test]
    fn test_http_request_dns_failure_returns_err() {
        // Unresolvable host = real transport-layer failure = Err.
        let result = http_request(
            "GET",
            "http://this-host-does-not-exist.invalid/",
            &HashMap::new(),
            None,
            false,
        );
        assert!(result.is_err(), "DNS failure must surface as Err");
    }

    #[test]
    fn test_extract_introspect_payload_happy_path() {
        let stdout = "noise line\n__SWAIG_TOOLS_BEGIN__\n{\"tools\":[]}\n__SWAIG_TOOLS_END__\nmore noise\n";
        let payload = extract_introspect_payload(stdout).unwrap();
        assert_eq!(payload, "{\"tools\":[]}");
    }

    #[test]
    fn test_extract_introspect_payload_missing_markers() {
        let stdout = "no markers anywhere";
        assert!(extract_introspect_payload(stdout).is_none());
    }

    #[test]
    fn test_extract_introspect_payload_partial_marker() {
        // BEGIN present, END missing — must return None, not garbage.
        let stdout = "__SWAIG_TOOLS_BEGIN__\n{\"tools\":[]}\n";
        assert!(extract_introspect_payload(stdout).is_none());
    }

    #[test]
    fn test_http_request_verbose_does_not_change_behavior() {
        // Verbose mode logs to stderr but does not change the result. Real
        // round-trip succeeds the same way it does in non-verbose mode.
        let (base, _captured, _h) = spawn_test_server(200, r#"{"ok":true}"#);
        let mut headers = build_headers(&None);
        headers.insert("Authorization".to_string(), "Basic dGVzdDp0ZXN0".to_string());
        let (status, body) = http_request(
            "POST",
            &format!("{base}/swaig"),
            &headers,
            Some("{\"key\":\"val\"}"),
            true, // verbose
        )
        .expect("verbose POST should round-trip");
        assert_eq!(status, 200);
        assert!(body.contains("\"ok\":true"));
    }
}
