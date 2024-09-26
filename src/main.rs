use clap::Parser;
use scraper::Scraper;
use tokio::time;

mod scraper;
// Define the command-line arguments structure
#[derive(Parser, Debug)]
#[clap(name = "Scraper CLI", about = "A simple web scraper CLI application.")]
struct Cli {
    /// Base URL for the scraping service
    #[clap(short, long)]
    base_url: String,

    /// Username for authentication
    #[clap(long)]
    client_id: String,

    /// Password for authentication
    #[clap(long)]
    client_secret: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let base_url = cli.base_url.clone();
    let client_id = cli.client_id.clone();
    let client_secret = cli.client_secret.clone();

    let credentials = scraper::Credentials::new(client_id, client_secret);
    let mut scraper = Scraper::new(base_url, credentials);

    loop {
        println!("Starting scraping run");
        scraper.scrape().await.unwrap();
        println!("Scrape finished, sleeping for 60 seconds");
        time::sleep(time::Duration::from_secs(60)).await;
    }
}
