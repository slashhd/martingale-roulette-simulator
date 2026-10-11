use martingale_sim::{
    api::{create_router, AppState},
    storage::StorageManager,
};
use std::{net::SocketAddr, sync::Arc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::var("MARTINGALE_PORT")
        .unwrap_or_else(|_| "3000".into())
        .parse::<u16>()?;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let csv_path = std::env::var_os("MARTINGALE_CSV_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "martingale_simulations.csv".into());
    let app = create_router(Arc::new(AppState::new(StorageManager::with_path(csv_path))));
    // Bind before reporting readiness: a conflicting port must produce a real error.
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Martingale Roulette Monte Carlo Simulator");
    println!("Open http://{} in your browser.", listener.local_addr()?);
    println!("Press Ctrl+C to stop the server.");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
