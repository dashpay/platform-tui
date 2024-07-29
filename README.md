# Terminal User Interface

The Dash Platform Terminal User Interface (TUI) is a Rust-based user interface for interacting with Dash Platform in the terminal. Its purpose is to enable users to perform all actions permitted by Dash Platform, which, broadly speaking, means broadcasting state transitions and querying the network.

The TUI can connect to any instance of a Dash Platform network, including the testnet, devnets, local networks, and soon, the mainnet. However, for now this readme will only cover connecting to a testnet, and steps to connect to a local network will be added soon. Further steps on running strategies, read [Paul Delucia's guide](https://www.dash.org/blog/strategy-tests-usage-guide/)

# Install

## Setup Dash Core

First, you need to run a Dash Core testnet node in order to connect to the testnet. Download the latest version of Dash Core [here](https://www.dash.org/downloads/#desktop). Run Dash Core and then configure the `dash.conf` file as follows, 
replacing `***` with a username and password of your choosing (find the `dash.conf` file by right-clicking the Dash Core icon and selecting "Open Wallet Configuration File"):

```conf
server=1
listen=1
rpcallowip=127.0.0.1
rpcuser=***
rpcpassword=***
testnet=1
```

Restart Dash Core for the changes to take effect.

## Setup TUI

### Clone repository

Clone the [TUI repo](https://github.com/dashpay/platform-tui):

```shell
git clone https://github.com/dashpay/platform-tui.git
```

Open the TUI repo in your terminal and install Rust

```shell
cd rs-platform-explorer`
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installing Rust, restart your terminal and navigate back to the platform-tui directory.

### Add the WebAssembly target to Rust

```shell
rustup target add wasm32-unknown-unknown`
```

### Install dependencies

Install build-essential tools, SSL development libraries, and Clang. On Ubuntu, use:

```shell
sudo apt install -y build-essential cmake libssl-dev pkg-config clang unzip`
```

On other Unix-like systems, use the equivalent package management commands.

### Install wasm-bindgen-cli

```shell
cargo install wasm-bindgen-cli@0.2.85`
```

### Install Protocol Buffers Compiler (protoc)

Download the appropriate protoc binary for your system, unzip, and install:

```shell
wget https://github.com/protocolbuffers/protobuf/releases/download/v26.1/protoc-26.1-linux-x86_64.zip`
sudo unzip protoc-*-linux-x86_64.zip -d /usr/local`
```

### Setup env file

Now, create a file named `.env` in the highest level of the TUI directory, and copy the contents of `.env.testnet` into it. Set the username and password in the `.env` file to the username and password in your Dash Core `wallet.conf` file and save.

### Build and run

Do `cargo run` to start the TUI:

```shell
cargo run
```

## Wallet operations

Now that we are inside the TUI, we need to load a wallet and an identity.

* Go to the Wallet screen and add a wallet by private key. You can generate a private key on [this website](https://passwordsgenerator.net/sha256-hash-generator/) by typing in a seed (it can be any random word or phrase), and then paste that key into the TUI. (take note: the key has to be in hexadecimal format!)
* After entering the private key, copy the Wallet receive address and paste it into the [testnet faucet](https://faucet.testnet.networks.dash.org/) to get some Dash funds. Use promo code `platform` to get 50 DASH (normally it only dispenses around 2-5).
* In the TUI, refresh the wallet balance until the funds appear.
* Register an identity and fund it with 1 DASH. This is more than enough for the example test we’ll be running.
* Refresh the identity balance until you see the funds appear.

## Strategy tests

Now, we’re ready to build and run a strategy. for further indication on how to do so, you can look [Paul Delucia's guide](https://www.dash.org/blog/strategy-tests-usage-guide/)
