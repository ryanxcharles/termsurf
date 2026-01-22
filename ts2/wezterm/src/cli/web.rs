use clap::{Parser, Subcommand};
use wezterm_client::client::Client;

#[derive(Debug, Parser, Clone)]
pub struct WebCommand {
    #[command(subcommand)]
    pub sub: WebSubCommand,
}

#[derive(Debug, Subcommand, Clone)]
pub enum WebSubCommand {
    /// Open a URL in a browser pane
    #[command(name = "open")]
    Open(WebOpen),
}

#[derive(Debug, Parser, Clone)]
pub struct WebOpen {
    /// The URL to open
    url: String,
}

impl WebCommand {
    pub async fn run(&self, client: Client) -> anyhow::Result<()> {
        match &self.sub {
            WebSubCommand::Open(cmd) => cmd.run(client).await,
        }
    }
}

impl WebOpen {
    pub async fn run(&self, client: Client) -> anyhow::Result<()> {
        let pane_id = client.resolve_pane_id(None).await?;
        let response = client
            .web_open(codec::WebOpen {
                pane_id,
                url: self.url.clone(),
            })
            .await?;
        println!("{}", response.message);
        Ok(())
    }
}
