# Configuration

The configuration is driven by a set of environment variables that can be defined either in your system environment or in a `.env` file in the project root.

## Required Environment Variables

The following environment variables are used to configure the project. Make sure to set them up as described:

### 1. `TARGET_CHAIN`

- **Description:**
  Whether to run locally, against the devnet or testnet.
- **Accepted Values:**
  - `"local"`: Runs a local node.
  - `"devnet"`: Connects to the Fuel Devnet at `https://devnet.fuel.network`.
  - `"testnet"`: Connects to the Fuel Testnet at `https://testnet.fuel.network`.

### 2. `DEV_KEY`

- **Description:**
  When `TARGET_CHAIN` is set to `"devnet"`, this variable must be set to the private key for the wallet used on the Devnet.

### 3. `TESTNET_KEY`

- **Description:**
  When `TARGET_CHAIN` is set to `"testnet"`, this variable must be set to the private key for the wallet used on the Testnet.

### 4. `FORCE_DEPLOY`

- **Description:**
  A boolean flag that indicates whether to force contract deployment even if a previous instance exists.
- **Accepted Values:**
  - `"true"` (case insensitive) to force deployment.
  - Any other value or absence of this variable will default to `false`.

### 5. `DEPLOY_IN_BLOBS`

- **Description:**
  A boolean flag that indicates whether the contract should be deployed as blobs (as a loader) or not.
- **Accepted Values:**
  - `"true"` (case insensitive) to deploy in blobs.
  - Any other value or absence of this variable will default to `false`.

## Example `.env` File

Below is an example of what your `.env` file might look like when targeting the devnet:

```env
TARGET_CHAIN=devnet
DEV_KEY=your_devnet_private_key_here
FORCE_DEPLOY=true
DEPLOY_IN_BLOBS=false
```

For a local setup, you only need to set `TARGET_CHAIN` to `local`:

```env
TARGET_CHAIN=local
FORCE_DEPLOY=false
DEPLOY_IN_BLOBS=false
```

## Running Tests

To run the tests for the project, simply execute:

```bash
cargo test -- --test-threads=1
```
