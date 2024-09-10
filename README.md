# Dash Platform Terminal User Interface

The Dash Platform Terminal User Interface (TUI) is a Rust-based user interface for interacting with Dash Platform in the terminal. Its purpose is to enable users to perform all actions permitted by Dash Platform, which, broadly speaking, means broadcasting state transitions and querying the network.

The TUI can connect to any instance of a Dash Platform network, including the mainnet, testnet, devnets, and local networks. The main [Install section](#install) of this readme covers connecting to mainnet and testnet, with a brief explanation of how to connect to a local network to follow. Check out the [TUI documentation](https://docs.dash.org/projects/platform/en/stable/docs/tutorials/tui/index.html), and for further steps on running strategy tests, read [Paul DeLucia's guide](https://www.dash.org/blog/strategy-tests-usage-guide/).

# Install

## Setup Dash Core

First, you need to run a Dash Core node in order to connect to mainnet or testnet. Download the latest version of Dash Core [here](https://www.dash.org/downloads/#desktop). Run Dash Core and then configure the `dash.conf` file as follows, 
replacing `***` with a username and password of your choosing (find the `dash.conf` file by right-clicking the Dash Core icon and selecting "Open Wallet Configuration File"):

```conf
server=1
listen=1
rpcallowip=127.0.0.1
rpcuser=***
rpcpassword=***
```

Put `testnet=1` if you're connecting to the testnet.

Restart Dash Core for the changes to take effect.

## Setup TUI

### Clone repository

Clone the [TUI repo](https://github.com/dashpay/platform-tui):

```shell
git clone https://github.com/dashpay/platform-tui.git
```

Open the TUI repo in your terminal:

```shell
cd rs-platform-explorer

Install Rust if you don't already have it

```shell
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
sudo apt update
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

Now, create a file named `.env` in the highest level of the TUI directory, and copy the contents of `.env.mainnet` or `.env.testnet` into it. Set the username and password in the `.env` file to the username and password in your Dash Core `dash.conf` file and save. Optionally, you can also put the private key of your Dash wallet into the `.env` file as well (hex or WIF format), otherwise you can load it into the TUI later inside the interface.

### Build and run

Do `cargo run` to start the TUI:

```shell
cargo run
```

### Connect to local network

Connecting to a local network follows almost the same steps as connecting to mainnet or testnet, with the following differences:

* You don't need to run Dash Core. Instead, you need to run a local Platform network. The steps to do this are in the [FAQ](https://github.com/dashpay/platform?tab=readme-ov-file#how-to-build-and-set-up-a-node-from-the-code-in-this-repo) of the Platform Readme.
* To get the username and password for the `.env` file, run `yarn dashmate config get core.rpc --config=local_seed` in the local Platform directory after you get a network running. Use the password for `dashmate` and `dashmate` as the username.

## DPNS Username Registration Tutorial

Now that we are inside the TUI, we will load a wallet, an identity, and register a DPNS username for the identity.

* Go to the Wallet screen and add a wallet by private key.
* If you don't already have a mainnet wallet, you can generate one on [paper.dash.org](https://paper.dash.org/). Save the address and private key (secret key) somewhere safe, then copy and paste the private key into the TUI. For all other networks, you can use [this website](https://passwordsgenerator.net/sha256-hash-generator/) to generate a new wallet private key if you don't already have one.
* Refresh the wallet balance.
* If your wallet is not yet funded, you'll need to send it some Dash before you can register an identity. After sending funds, you'll need to refresh the wallet balance again until the funds appear.
* Now go back to the Main screen and then to the Identities screen.
* Here, you can either load an existing identity if you have its ID and private keys, or register a new one.
* If you register a new one, fund it with 0.01 DASH. This should be more than enough to register a (uncontested) DPNS username. The private keys of the newly created identity will be logged to `platform-tui/supporting_files/new_identity_private_keys.log`. Be sure to save a copy of that file somewhere safe.
* After registering or loading an identity, refresh the identity balance until you see the funds appear.
* Now, you can register a DPNS username for the identity.
* After registering the username from the Identities screen, you can navigate back to the Main screen, then the DPNS screen, and from there, query the names for the selected identity, and you should see the name there that you just registered, fetched from the Dash Platform state.
