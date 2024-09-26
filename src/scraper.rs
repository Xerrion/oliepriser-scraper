use chrono::DateTime;
use futures::stream;
use futures::stream::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{Error, ErrorKind};
use std::sync::Arc;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Token {
    access_token: String,
    token_type: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct Providers {
    id: i32,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Provider {
    id: i32,
    name: String,
    url: String,
    html_element: String,
}

pub(crate) struct Credentials {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    token: Token,
}

impl Credentials {
    pub(crate) fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            token: Token {
                access_token: "".to_string(),
                token_type: "".to_string(),
            },
        }
    }
}

pub(crate) struct Scraper {
    providers: Vec<Providers>,
    pub(crate) credentials: Credentials,
    client: Client,
    base_url: String,
    run_start: DateTime<chrono::Utc>,
    run_end: Option<DateTime<chrono::Utc>>,
}

impl Scraper {
    pub(crate) fn new(base_url: String, credentials: Credentials) -> Self {
        Self {
            providers: vec![],
            client: Client::new(),
            credentials,
            base_url,
            run_start: chrono::Utc::now(),
            run_end: None,
        }
    }

    async fn post_run(&self) -> Result<(), reqwest::Error> {
        let now = chrono::Utc::now();
        let json_body = json!({
            "start_time": self.run_start,
            "end_time": self.run_end.unwrap_or(now),
        });
        let url = Url::parse(&format!("{}/scraping_runs", self.base_url)).unwrap();
        self.client.post(url).json(&json_body).send().await?;
        Ok(())
    }
    async fn fetch_providers(&self) -> Result<Vec<Providers>, Error> {
        let url = Url::parse(&format!("{}/scraping_runs/providers", self.base_url)).unwrap();
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?;

        if status.is_success() {
            let providers = serde_json::from_str::<Vec<Providers>>(&body)?;
            Ok(providers)
        } else {
            Err(Error::new(ErrorKind::Other, "Failed to fetch providers"))
        }
    }

    async fn add_price_for_provider(
        &self,
        provider_id: i32,
        price: f64,
    ) -> Result<(), reqwest::Error> {
        let url = Url::parse(&format!(
            "{}/providers/{}/prices",
            self.base_url, provider_id
        ))
        .unwrap();
        let json_price = json!({ "price": price });
        let response = self.client.post(url).json(&json_price).send().await?;
        let status = response.status();

        if response.status().is_success() {
            let body = response.text().await?;
            println!("Added price for provider {}: {}", provider_id, body);
        } else {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "No response body".to_string());
            eprintln!(
                "Failed to add price for provider {}: {} {}",
                provider_id, status, body
            );
        }
        Ok(())
    }

    fn sanitize_price_string(&self, price_string: String) -> Result<f64, String> {
        // Remove unwanted characters and whitespace
        let sanitized: String = price_string
            .replace("kr.", "")
            .replace(",-", "")
            .replace('.', "")
            .replace(',', ".")
            .replace(|c: char| c.is_whitespace(), "");

        sanitized
            .parse::<f64>()
            .map_err(|e| format!("Failed to parse price: {}", e))
    }

    async fn handle_scraping(&self) -> Result<(), reqwest::Error> {
        let self_arc = Arc::new(self); // Wrap self in Arc

        let tasks = self_arc.providers.iter().map(|provider| {
            let client = self_arc.client.clone(); // Clone Arc for each async block

            let self_arc_clone = Arc::clone(&self_arc); // Clone Arc for usage in the async block
            async move {
                let provider = self.get_provider(provider, &client).await?;
                println!("Scraping provider: {}", provider.name);

                let selector = Selector::parse(&provider.html_element).unwrap();
                let provider_url = Url::parse(&provider.url).unwrap();

                let response = client.get(provider_url).send().await?;
                let body = response.text().await?;
                let document = Html::parse_document(&body);
                self_arc_clone
                    .extract_price(provider, document, &selector)
                    .await; // Call using the cloned Arc
                Ok::<_, reqwest::Error>(())
            }
        });

        let results: Vec<Result<(), reqwest::Error>> = stream::iter(tasks)
            .buffer_unordered(10) // Set a concurrency limit
            .collect()
            .await;

        for result in results {
            result?;
        }
        Ok(())
    }

    async fn get_provider(
        &self,
        provider: &Providers,
        client: &Client,
    ) -> Result<Provider, reqwest::Error> {
        let provider = client
            .get(Url::parse(&format!("{}/providers/{}", self.base_url, provider.id)).unwrap())
            .send()
            .await?
            .json::<Provider>()
            .await?;

        Ok(provider)
    }

    async fn extract_price(&self, provider: Provider, document: Html, selector: &Selector) {
        for element in document.select(selector) {
            let price_string = element.text().collect::<String>();
            match self.sanitize_price_string(price_string) {
                Ok(price) if price > 0.0 => {
                    if let Err(e) = self.add_price_for_provider(provider.id, price).await {
                        eprintln!("Error adding price for provider {}: {}", provider.name, e);
                    }
                    return; // Price found, exit the function
                }
                _ => {}
            }
        }
        println!("No price found for provider: {}", provider.name);
    }

    async fn get_token(&mut self) -> Result<Token, reqwest::Error> {
        let url = Url::parse(&format!("{}{}", self.base_url, "/auth/login")).unwrap();
        let response = self
            .client
            .post(url)
            .json(&json!({
                "client_id": self.credentials.client_id,
                "client_secret": self.credentials.client_secret,
            }))
            .send()
            .await?
            .json()
            .await?;

        Ok(response)
    }

    async fn configure_client(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::new();
        let auth_value = format!(
            "{} {}",
            self.credentials.token.token_type, self.credentials.token.access_token
        );
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_value)?);

        self.client = Client::builder().default_headers(headers).build()?;
        Ok(())
    }

    pub(crate) async fn scrape(&mut self) -> Result<(), reqwest::Error> {
        self.run_start = chrono::Utc::now();
        self.credentials.token = self.get_token().await.unwrap();
        self.configure_client().await.unwrap();
        self.providers = self.fetch_providers().await.unwrap();
        self.handle_scraping().await?;
        self.run_end = Some(chrono::Utc::now());
        self.post_run().await?;
        Ok(())
    }
}
