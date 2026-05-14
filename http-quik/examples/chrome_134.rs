use quik::{Client, Platform};

#[tokio::main]
async fn main() -> quik::Result<()> {
    tracing_subscriber::fmt::init();

    // 1. Setup the High-Level Client (The Pool)
    // This replaces wreq::Client::new()
    let client = Client::builder()
        .profile(quik::profile::chrome_134::profile(Platform::MacOsArm))
        .build()?;

    let url = "https://tls.peet.ws/api/all";

    println!("Requesting fingerprint analysis via High-Level Pool API...");
    // 2. Just pass the URL. Quik handles DNS, TCP, TLS, and H2 handshakes.
    let response = client.get(url).await?;

    // 3. Parse JSON (Response handles stream collection and decompression)
    let json: serde_json::Value = response.json().await?;

    println!("--- Final Parity Verification ---");
    println!("Browser Detected: {}", json["browser"]);
    println!("JA4: {}", json["tls"]["ja4"]);
    println!("Akamai: {}", json["http2"]["akamai_fingerprint"]);
    println!("----------------------------------");

    if json["http2"]["akamai_fingerprint"].as_str()
        == Some("1:65536;2:0;4:6291456;6:262144|15663105|0|m,a,s,p")
    {
        println!("✅ AKAMAI PARITY REACHED");
    }

    if json["tls"]["ja4"]
        .as_str()
        .unwrap_or("")
        .starts_with("t13d1516h2")
    {
        println!("✅ JA4 SIGNALS REACHED (16 Extensions)");
    }

    Ok(())
}
