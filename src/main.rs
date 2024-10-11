use clap::Parser;
use credentials::Credentials;
use scraper::Scraper;
use tokio::time;

mod credentials;
mod scraper;
// Define the command-line arguments structure
#[derive(Parser, Debug)]
#[clap(name = "Scraper CLI", about = "A simple web scraper CLI application.")]
struct Cli {
    /// Base URL for the API
    #[clap(short, long)]
    base_api_url: String,

    /// Username for authentication
    #[clap(long)]
    client_id: String,

    /// Password for authentication
    #[clap(long)]
    client_secret: String,
}

#[tokio::main]
async fn main() {
    // Parse the command-line arguments
    let cli = Cli::parse();
    let base_api_url = cli.base_api_url.clone();
    let client_id = cli.client_id.clone();
    let client_secret = cli.client_secret.clone();

    // Create a new Scraper instance
    let credentials = Credentials::new(client_id, client_secret);
    let mut scraper = Scraper::new(base_api_url, credentials);

    // Start the scraping loop
    loop {
        println!("Starting scraping run");
        scraper.run().await.unwrap();
        println!("Scrape finished, sleeping for 60 seconds");
        time::sleep(time::Duration::from_secs(60)).await;
    }
}
