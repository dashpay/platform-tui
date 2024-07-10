First, you need to run a Dash Core testnet node in order to connect to the testnet. Download the latest version of Dash Core [here](https://www.dash.org/downloads/#desktop). Run Dash Core and then configure the `dash.conf` file as follows, 
replacing `***` with a username and password of your choosing (find the `dash.conf` file by right-clicking the Dash Core icon and selecting "Open Wallet Configuration File"):

* `server=1`
* `listen=1`
* `rpcallowip=127.0.0.1`
* `rpcuser=***`
* `rpcpassword=***`
* `testnet=1`

Restart Dash Core for the changes to take effect.

-------------

Next, clone the [TUI repo](https://github.com/dashpay/rs-platform-explorer):

`git clone https://github.com/dashpay/rs-platform-explorer.git`

-------------

Open the TUI repo in your terminal and install Rust

`cd rs-platform-explorer`

`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`


After installing Rust, restart your terminal and navigate back to rs-platform-explorer.

-------------

Add the WebAssembly target to Rust


`rustup target add wasm32-unknown-unknown`

-------------

Install build-essential tools, SSL development libraries, and Clang. On Ubuntu, use:

`sudo apt install -y build-essential libssl-dev pkg-config clang`

On other Unix-like systems, use the equivalent package management commands.

-------------
Install wasm-bindgen-cli:

`cargo install wasm-bindgen-cli@0.2.85`

-------------
Install Protocol Buffers Compiler (protoc):

Download the appropriate protoc binary for your system:

`wget https://github.com/protocolbuffers/protobuf/releases/download/v26.1/protoc-26.1-linux-x86_64.zip`

Install unzip if not already installed:

`sudo apt install unzip`

Unzip and install `protoc`:

`sudo unzip protoc-*-linux-x86_64.zip -d /usr/local`

-------------
Install CMake:

`sudo apt update`
`sudo apt install cmake`

-------------
Now, create a file named `.env` in the highest level of the TUI directory, and copy the contents of `.env.testnet` into it

-------------
Set the username and password in the `.env` file to the username and password in your Dash Core `wallet.conf` file and save

-------------
Do `cargo run` to start the TUI:

`cargo run`

-------------
Now that we are inside the TUI, we need to load a wallet and an identity.


* Go to the Wallet screen and add a wallet by private key. You can generate a private key on [this website](https://passwordsgenerator.net/sha256-hash-generator/) by typing in a seed (it can be any random word or phrase), and then paste that key into the TUI. (take note: the key has to be in hexadecimal format!)
* After entering the private key, copy the Wallet receive address and paste it into the [testnet faucet](https://faucet.testnet.networks.dash.org/) to get some Dash funds. Use promo code `platform` to get 50 DASH (normally it only dispenses around 2-5).
* In the TUI, refresh the wallet balance until the funds appear.
* Register an identity and fund it with 1 DASH. This is more than enough for the example test we’ll be running.
* Refresh the identity balance until you see the funds appear.

-------------

Now, we’re ready to build and run a strategy. for further indication on how to do so, you can look [Paul Delucia's guide](https://www.dash.org/blog/strategy-tests-usage-guide/)

-------------
