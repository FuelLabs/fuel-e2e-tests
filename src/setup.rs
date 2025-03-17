use color_eyre::{eyre::Context, Result, Section};
use dotenv::dotenv;
use fuels::{
    accounts::provider::Provider, crypto::SecretKey, test_helpers::launch_provider_and_get_wallet,
};

#[cfg(feature = "fuels_lts_70")]
pub type Wallet = fuels::accounts::wallet::WalletUnlocked;

#[cfg(feature = "fuels_71")]
pub type Wallet = fuels::accounts::wallet::Wallet;

#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// Whether to force deployment even if we already have an instance of the contract deployed.
    pub force_deploy: bool,
    /// Whether to deploy the contract in blobs (as a loader) or not.
    pub deploy_in_blobs: bool,
}

#[derive(Debug, Clone)]
pub struct Setup {
    /// With funds, taken from ENV
    pub wallet: Wallet,
    /// Tweaking how contracts should be deployed
    pub deploy_config: DeployConfig,
}

pub async fn init() -> Result<Setup> {
    // affects global state so it can fail if already set
    let _ = color_eyre::install();

    // It can fail if there is no file, that's ok.
    let _ = dotenv();

    let chain = read_chain()?;

    let wallet = chain.wallet().await?;

    let force_deploy = check_boolean_env("FORCE_DEPLOY")?;
    let deploy_in_blobs = check_boolean_env("DEPLOY_IN_BLOBS")?;

    Ok(Setup {
        wallet,
        deploy_config: DeployConfig {
            force_deploy,
            deploy_in_blobs,
        },
    })
}

fn check_boolean_env(env: &str) -> Result<bool> {
    let Some(env) = read_env(env).ok() else {
        return Ok(false);
    };

    Ok(env.to_lowercase() == "true")
}

enum Chain {
    Local,
    Devnet,
    Testnet,
}

impl Chain {
    async fn wallet(&self) -> Result<Wallet> {
        let wallet_from_env_key = |env_var: &'static str, url: &'static str| async move {
            let provider = Provider::connect(url)
                .await
                .wrap_err_with(|| format!("failed to connect to {url}"))?;

            let key: SecretKey = read_env(env_var)?
                .parse()
                .wrap_err("given private key is invalid")?;

            #[cfg(feature = "fuels_lts_70")]
            let wallet = Wallet::new_from_private_key(key, Some(provider));
            #[cfg(feature = "fuels_71")]
            let wallet = {
                let signer = fuels::accounts::signers::private_key::PrivateKeySigner::new(key);
                Wallet::new(signer, provider)
            };

            Result::<_>::Ok(wallet)
        };

        let wallet = match self {
            Chain::Devnet => wallet_from_env_key("DEV_KEY", "https://devnet.fuel.network").await?,
            Chain::Testnet => {
                wallet_from_env_key("TESTNET_KEY", "https://testnet.fuel.network").await?
            }
            Chain::Local => launch_provider_and_get_wallet().await?,
        };

        Ok(wallet)
    }
}

fn read_env(name: &str) -> Result<String> {
    let msg =
        format!("did you setup {name} env variable? add them in a .env file e.g. {name}=abcd...");
    std::env::var(name).suggestion(msg)
}

fn read_chain() -> Result<Chain> {
    let var = read_env("TARGET_CHAIN")?;

    let env = match var.as_str() {
        "devnet" => Chain::Devnet,
        "testnet" => Chain::Testnet,
        "local" => Chain::Local,
        env => {
            return Err(color_eyre::eyre::eyre!("invalid target chain value: {env}")
                .suggestion("use 'devnet' or 'testnet'"))
        }
    };

    Ok(env)
}
