use game_data::{BUNDLED_FIXTURES, Catalog};
use std::sync::Arc;
use web_app::{
    AppState, accounts::Accounts, auth::IdentityProvider, auth::placeholder::PlaceholderProvider,
    config::Config, persistence, router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env().map_err(std::io::Error::other)?;
    let catalog = Catalog::from_json(BUNDLED_FIXTURES).map_err(std::io::Error::other)?;
    let pool = persistence::connect(&config.database_url)
        .await
        .map_err(std::io::Error::other)?;
    persistence::require_schema(&pool)
        .await
        .map_err(std::io::Error::other)?;

    let accounts =
        Accounts::new(pool, config.session_ttl_seconds).map_err(std::io::Error::other)?;
    let provider = build_provider(&config);
    let listener = tokio::net::TcpListener::bind(&config.bind).await?;
    let auth_mode = if config.dev_auth {
        "development placeholder authentication (NOT SECURE)"
    } else {
        "authentication disabled"
    };
    println!(
        "Karta Yaxu — http://{} ({auth_mode})",
        listener.local_addr()?
    );
    if config.dev_auth {
        eprintln!("WARNING: placeholder authentication is enabled; do not expose this server.");
    }

    let state = AppState::new(
        catalog,
        accounts,
        provider,
        config.public_origin.clone(),
        config.cookie_secure,
    );
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async move {
            tokio::select! { _ = interrupt.recv() => {}, _ = terminate.recv() => {} }
        })
        .await?;
    Ok(())
}

/// The single place that selects the identity provider. Replacing the
/// provider changes this selection and adds its credential-specific HTTP bootstrap.
fn build_provider(config: &Config) -> Option<Arc<dyn IdentityProvider>> {
    if config.dev_auth {
        Some(Arc::new(PlaceholderProvider::new()))
    } else {
        None
    }
}
