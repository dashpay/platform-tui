#!/bin/bash

set -e

# Check if the devnet name is provided as an argument
if [ -z "$1" ]; then
    echo "Usage: $0 <devnet-name>"
    exit 1
fi

DEVNET_NAME=$1

# Define file paths
ENV_LOCAL=".env.local"
ENV=".env"
BACKUP_DIR=".env.backups"
BACKUP_ENV="$BACKUP_DIR/.env.backup_$(date +%Y%m%d_%H%M%S)"
PLATFORM_ENV="../platform/packages/platform-test-suite/.env"
PLATFORM_DIR="../platform"
CURRENT_DIR=$(pwd)
NETWORK_CONFIG_REPO="../dash-network-configs"
DEVNET_CONFIG_FILE="$NETWORK_CONFIG_REPO/devnet-$DEVNET_NAME.yml"
INVENTORY_FILE="$NETWORK_CONFIG_REPO/devnet-$DEVNET_NAME.inventory"

# Ensure the dash-network-configs repository is up to date
if [ ! -d "$NETWORK_CONFIG_REPO" ]; then
    echo "Cloning dash-network-configs repository..."
    git clone https://github.com/dashpay/dash-network-configs $NETWORK_CONFIG_REPO
else
    echo "Updating dash-network-configs repository..."
    cd $NETWORK_CONFIG_REPO
    git checkout master
    git pull origin master
    cd $CURRENT_DIR
fi

# Check if the inventory file exists after the update
if [ ! -f "$INVENTORY_FILE" ]; then
    echo "Inventory file $INVENTORY_FILE not found."
    exit 1
fi

# Determine the location of dash.conf based on the OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    DASH_CONF_PATH="$HOME/.dashcore/dash.conf"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    DASH_CONF_PATH="$HOME/Library/Application Support/DashCore/dash.conf"
elif [[ "$OSTYPE" == "cygwin" || "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    DASH_CONF_PATH="$APPDATA/DashCore/dash.conf"
else
    echo "Unsupported OS type: $OSTYPE"
    exit 1
fi

# Check if the dash.conf file exists
if [ ! -f "$DASH_CONF_PATH" ]; then
    echo "dash.conf not found at $DASH_CONF_PATH"
    exit 1
fi

# Check if the devnet name matches the one in the dash.conf file
DEVNET_IN_CONF=$(grep -o 'devnet=.*' "$DASH_CONF_PATH" | cut -d '=' -f2)

if [ "$DEVNET_IN_CONF" != "$DEVNET_NAME" ]; then
    echo "The devnet name in dash.conf ($DEVNET_IN_CONF) does not match the provided devnet name ($DEVNET_NAME)"
    exit 1
fi

# Extract RPC credentials from dash.conf
echo "Extracting RPC credentials from $DASH_CONF_PATH..."
RPC_USER=$(grep -o 'rpcuser=.*' "$DASH_CONF_PATH" | cut -d '=' -f2)
RPC_PASSWORD=$(grep -o 'rpcpassword=.*' "$DASH_CONF_PATH" | cut -d '=' -f2)

# Check if the credentials were successfully extracted
if [ -z "$RPC_USER" ] || [ -z "$RPC_PASSWORD" ]; then
    echo "Failed to extract RPC credentials from dash.conf"
    exit 1
fi

# Create backup directory if it doesn't exist
if [ ! -d $BACKUP_DIR ]; then
    echo "Creating backup directory $BACKUP_DIR"
    mkdir -p $BACKUP_DIR
fi

# Backup the current .env file if it exists
if [ -f $ENV ]; then
    echo "Backing up current .env to $BACKUP_ENV"
    cp $ENV $BACKUP_ENV
fi

# Move .env.local to .env
echo "Moving $ENV_LOCAL to $ENV"
cp $ENV_LOCAL $ENV

# Change to the platform directory
cd $PLATFORM_DIR

# Extract FAUCET_1_PRIVATE_KEY from the devnet YAML file
echo "Extracting FAUCET_1_PRIVATE_KEY from $DEVNET_CONFIG_FILE..."
FAUCET_1_PRIVATE_KEY=$(grep -o 'faucet_privkey: .*' "$DEVNET_CONFIG_FILE" | cut -d ' ' -f2)

# Check if FAUCET_1_PRIVATE_KEY was found
if [ -z "$FAUCET_1_PRIVATE_KEY" ]; then
    echo "Failed to extract FAUCET_1_PRIVATE_KEY. Ensure that the key is present in $DEVNET_CONFIG_FILE."
    exit 1
fi

# Change back to the original directory
cd $CURRENT_DIR

# Extract the RPC port from dash.conf
echo "Extracting RPC port from $DASH_CONF_PATH..."
RPC_PORT=$(grep -o 'rpcport=.*' "$DASH_CONF_PATH" | cut -d '=' -f2)

# Check if the RPC port was successfully extracted
if [ -z "$RPC_PORT" ]; then
    echo "Failed to extract RPC port from dash.conf. Setting to default 20302."
    RPC_PORT="20302"  # Default value if not set in dash.conf
fi

# Extract 5 random HP Masternodes from the inventory file using public IPs
echo "Selecting 5 random HP Masternodes public IPs from $INVENTORY_FILE..."
HP_MASTERNODES=$(grep 'hp-masternode' $INVENTORY_FILE | awk '{print $4}' | cut -d '=' -f2 | shuf -n 5)

# Convert the selected IPs into the required DAPI URL format
DAPI_ADDRESSES=$(echo $HP_MASTERNODES | sed 's/ /:1443,http:\/\//g')
DAPI_ADDRESSES="http://$DAPI_ADDRESSES:1443"

# Set EXPLORER_INSIGHT_API_URL based on the network name
EXPLORER_INSIGHT_API_URL="http://insight.$DEVNET_NAME.networks.dash.org:3001/insight-api"

# Update the .env file
echo "Updating .env file with extracted values..."
sed -i.bak "s/^EXPLORER_CORE_RPC_USER=.*/EXPLORER_CORE_RPC_USER=$RPC_USER/" $ENV
sed -i.bak "s/^EXPLORER_CORE_RPC_PASSWORD=.*/EXPLORER_CORE_RPC_PASSWORD=$RPC_PASSWORD/" $ENV
sed -i.bak "s/^EXPLORER_WALLET_PRIVATE_KEY=.*/EXPLORER_WALLET_PRIVATE_KEY=$FAUCET_1_PRIVATE_KEY/" $ENV
sed -i.bak "s/^EXPLORER_NETWORK=.*/EXPLORER_NETWORK=devnet/" $ENV
sed -i.bak "s|^EXPLORER_DAPI_ADDRESSES=.*|EXPLORER_DAPI_ADDRESSES=$DAPI_ADDRESSES|" $ENV
sed -i.bak "s/^EXPLORER_CORE_RPC_PORT=.*/EXPLORER_CORE_RPC_PORT=$RPC_PORT/" $ENV
sed -i.bak "s|^EXPLORER_INSIGHT_API_URL=.*|EXPLORER_INSIGHT_API_URL=$EXPLORER_INSIGHT_API_URL|" $ENV

# Output the final .env file for verification
echo "Updated .env file:"
cat $ENV

echo "Process completed successfully."