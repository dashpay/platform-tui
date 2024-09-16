#!/bin/bash

set -e

# Define file paths
ENV_LOCAL=".env.local"
ENV=".env"
BACKUP_DIR=".env.backups"
BACKUP_ENV="$BACKUP_DIR/.env.backup_$(date +%Y%m%d_%H%M%S)"
PLATFORM_ENV="../platform/packages/platform-test-suite/.env"
PLATFORM_DIR="../platform"
CURRENT_DIR=$(pwd)

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

# Get dashmate config and store it in a variable
echo "Getting dashmate config..."
DASHMATE_CONFIG=$(yarn dashmate config get core.rpc --config=local_seed)

# Check if the command was successful
if [ $? -ne 0 ]; then
    echo "Failed to get dashmate config."
    echo "DASHMATE_CONFIG was: $DASHMATE_CONFIG"
    exit 1
fi

# Extract the relevant portion containing the dashmate password
DASHMATE_SECTION=$(echo "$DASHMATE_CONFIG" | sed -n '/dashmate: {/p' | head -1)

# Print the extracted dashmate section
echo "DASHMATE_SECTION: $DASHMATE_SECTION"

# Extract the password from the DASHMATE_SECTION
DASHMATE_PASSWORD=$(echo "$DASHMATE_SECTION" | awk -F 'password: ' '{print $2}' | awk -F ',' '{print $1}' | tr -d "'\"")

# Print the extracted password
echo "DASHMATE_PASSWORD: $DASHMATE_PASSWORD"

# Change back to the original directory
cd $CURRENT_DIR

# Extract FAUCET_1_PRIVATE_KEY from platform-test-suite .env
echo "Extracting FAUCET_1_PRIVATE_KEY from $PLATFORM_ENV..."
FAUCET_1_PRIVATE_KEY=$(grep -o 'FAUCET_1_PRIVATE_KEY=.*' $PLATFORM_ENV | cut -d '=' -f2)

# Check if FAUCET_1_PRIVATE_KEY was found
if [ -z "$FAUCET_1_PRIVATE_KEY" ]; then
    echo "Failed to extract FAUCET_1_PRIVATE_KEY"
    echo "PLATFORM_ENV content was: $(cat $PLATFORM_ENV)"
    exit 1
fi

# Update the .env file
echo "Updating .env file with extracted values..."
sed -i.bak "s/^EXPLORER_CORE_RPC_USER=.*/EXPLORER_CORE_RPC_USER=dashmate/" $ENV
sed -i.bak "s/^EXPLORER_CORE_RPC_PASSWORD=.*/EXPLORER_CORE_RPC_PASSWORD=$DASHMATE_PASSWORD/" $ENV
sed -i.bak "s/^EXPLORER_WALLET_PRIVATE_KEY=.*/EXPLORER_WALLET_PRIVATE_KEY=\"$FAUCET_1_PRIVATE_KEY\"/" $ENV

# Output the final .env file for verification
echo "Updated .env file:"
cat $ENV

echo "Process completed successfully."