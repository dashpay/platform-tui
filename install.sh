#!/bin/bash

# Function to check if a command exists
command_exists () {
    command -v "$1" >/dev/null 2>&1 ;
}

echo "Starting Dash Platform TUI Setup..."

# Update system and install dependencies
echo "Updating system and installing dependencies..."
sudo apt update

echo "Installing build-essential, cmake, libssl-dev, pkg-config, clang, unzip..."
sudo apt install -y build-essential cmake libssl-dev pkg-config clang unzip

# Check if Rust is installed, otherwise install it
if ! command_exists rustc ; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
else
    echo "Rust is already installed."
fi

# Add WebAssembly target to Rust
echo "Adding WebAssembly target to Rust..."
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
if ! command_exists wasm-bindgen ; then
    echo "Installing wasm-bindgen-cli..."
    cargo install wasm-bindgen-cli@0.2.85
else
    echo "wasm-bindgen-cli is already installed."
fi

# Install Protocol Buffers Compiler (protoc)
if ! command_exists protoc ; then
    echo "Installing Protocol Buffers Compiler (protoc)..."
    wget https://github.com/protocolbuffers/protobuf/releases/download/v26.1/protoc-26.1-linux-x86_64.zip
    sudo unzip protoc-26.1-linux-x86_64.zip -d /usr/local
    rm protoc-26.1-linux-x86_64.zip
else
    echo "protoc is already installed."
fi

# Let the user select the network
echo "Select the network you want to connect to:"
echo "1) Mainnet"
echo "2) Testnet"
echo "3) Local network"
read -p "Enter the number corresponding to your choice: " network_choice

# Set up the .env file based on the network selection
case $network_choice in
    1)
        echo "You selected Mainnet. Creating .env file for Mainnet..."
        cp .env.mainnet .env
        ;;
    2)
        echo "You selected Testnet. Creating .env file for Testnet..."
        cp .env.testnet .env
        ;;
    3)
        echo "You selected Local Network. Creating .env file for Local Network..."
        cp .env.local .env
        ;;
    *)
        echo "Invalid selection. Exiting..."
        exit 1
        ;;
esac

# Ask for RPC username and password
read -p "Enter your Dash Core RPC username: " rpcuser
read -sp "Enter your Dash Core RPC password: " rpcpassword
echo ""

# Update .env file with user-provided credentials
sed -i "s/^RPC_USER=.*/RPC_USER=$rpcuser/" .env
sed -i "s/^RPC_PASSWORD=.*/RPC_PASSWORD=$rpcpassword/" .env

# Optionally, ask for a private key (optional)
read -p "Enter your Dash wallet private key (or press enter to skip): " privatekey
if [ -n "$privatekey" ]; then
    sed -i "s/^PRIVATE_KEY=.*/PRIVATE_KEY=$privatekey/" .env
fi

# For local network, provide additional instructions
if [ "$network_choice" -eq 3 ]; then
    echo "For local network, remember to run your local Platform instance and use 'yarn dashmate config get core.rpc --config=local_seed' to get the correct username and password."
fi

# Build and run the TUI
echo "Building and running the Dash Platform TUI..."
cargo run

echo "Setup complete! You can now use the Dash Platform TUI."
