use multiplayer_server::{app, Host};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Safe development default. Set BIND_ADDR explicitly to expose on a LAN.
    let address = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&address).await?;
    println!(
        "Multiplayer development host: http://{}",
        listener.local_addr()?
    );
    let host = Host::default();
    let shutdown_host = host.clone();
    let server = axum::serve(listener, app(host)).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    });
    tokio::select! {
        result = server => result?,
        _ = shutdown_host.run_ticks() => {},
    }
    Ok(())
}
