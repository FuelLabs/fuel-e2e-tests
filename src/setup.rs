use color_eyre::{eyre::Context, Result, Section};
use dotenv::dotenv;
use fuels::{
    accounts::{provider::Provider, wallet::WalletUnlocked},
    crypto::SecretKey,
    test_helpers::launch_provider_and_get_wallet,
};

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
    pub wallet: WalletUnlocked,
    /// Tweaking how contracts should be deployed
    pub deploy_config: DeployConfig,
}

pub async fn init() -> Result<Setup> {
    color_eyre::install()?;

    // It can fail if there is no file, that's ok.
    let _ = dotenv();

    let chain = read_chain()?;

    let mut wallet = chain.wallet().await?;

    chain.populate_provider(&mut wallet).await?;

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
    async fn populate_provider(&self, wallet: &mut WalletUnlocked) -> Result<()> {
        let connect_to_url = |url: &'static str| async move {
            Provider::connect(url)
                .await
                .wrap_err_with(|| format!("failed to connect to {url}"))
        };

        match self {
            Chain::Devnet => {
                let provider = connect_to_url("https://devnet.fuel.network").await?;
                wallet.set_provider(provider);
            }
            Chain::Testnet => {
                let provider = connect_to_url("https://testnet.fuel.network").await?;
                wallet.set_provider(provider);
            }
            Chain::Local => {}
        }

        Ok(())
    }

    async fn wallet(&self) -> Result<WalletUnlocked> {
        let wallet_from_env_key = |env_var: &str| -> Result<WalletUnlocked> {
            let key: SecretKey = read_env(env_var)?
                .parse()
                .wrap_err("given private key is invalid")?;

            Ok(WalletUnlocked::new_from_private_key(key, None))
        };

        let wallet = match self {
            Chain::Devnet => wallet_from_env_key("DEV_KEY")?,
            Chain::Testnet => wallet_from_env_key("TESTNET_KEY")?,
            Chain::Local => launch_provider_and_get_wallet().await?,
        };

        Ok(wallet)
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
        "local" => Chain::Local,
        env => {
            return Err(color_eyre::eyre::eyre!("invalid target chain value: {env}")
                .suggestion("use 'devnet' or 'testnet'"))
        }
    };

    Ok(env)
}
