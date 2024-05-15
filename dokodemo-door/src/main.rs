#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tracing_subscriber::EnvFilter::from_env().
    println!("{}", "hello dokodemo door");
    Ok(())
}

pub async fn run_server() -> anyhow::Result<()> {
    Ok(())
}