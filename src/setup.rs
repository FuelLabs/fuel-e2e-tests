use color_eyre::{eyre::Context, Result, Section};
use dotenv::dotenv;
use fuels::{
    accounts::{provider::Provider, wallet::WalletUnlocked},
    crypto::SecretKey,
};

pub async fn init() -> Result<WalletUnlocked> {
    color_eyre::install()?;
    dotenv()?;

    let chain = read_chain()?;

    let mut wallet = chain.wallet()?;

    let provider = chain.provider().await?;
    wallet.set_provider(provider);

    Ok(wallet)
}

enum Chain {
    Devnet,
    Testnet,
}

impl Chain {
    fn url(&self) -> &'static str {
        match self {
            Chain::Devnet => "https://devnet.fuel.network",
            Chain::Testnet => "https://testnet.fuel.network",
        }
    }

    async fn provider(&self) -> Result<Provider> {
        let url = self.url();
        Provider::connect(url)
            .await
            .wrap_err_with(|| format!("failed to connect to {url}"))
    }

    fn wallet(&self) -> Result<WalletUnlocked> {
        let key_env_var = match self {
            Chain::Devnet => "DEV_KEY",
            Chain::Testnet => "TESTNET_KEY",
        };

        let key: SecretKey = read_env(key_env_var)?
            .parse()
            .wrap_err("given private key is invalid")?;

        Ok(WalletUnlocked::new_from_private_key(key, None))
    }
}

fn read_env(name: &str) -> Result<String> {
    let msg = format!(
        "did you setup {name} env variable? add them in a .env file e.g. {name}=abcd..."
    );
    std::env::var(name).suggestion(msg)
}

fn read_chain() -> Result<Chain> {
    let var = read_env("TARGET_CHAIN")?;

    let env = match var.as_str() {
        "devnet" => Chain::Devnet,
        "testnet" => Chain::Testnet,
        env => {
            return Err(color_eyre::eyre::eyre!("invalid target chain value: {env}")
                .suggestion("use 'devnet' or 'testnet'"))
        }
    };

    Ok(env)
}
