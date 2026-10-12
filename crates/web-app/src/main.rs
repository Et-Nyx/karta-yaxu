use game_data::{BUNDLED_FIXTURES, Catalog};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let catalog = Catalog::from_json(BUNDLED_FIXTURES).map_err(std::io::Error::other)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    println!(
        "Karta Yaxu — http://{} (local placeholder preview)",
        listener.local_addr()?
    );
    axum::serve(listener, web_app::router(catalog))
        .with_graceful_shutdown(async move {
            tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
        })
        .await?;
    Ok(())
}
