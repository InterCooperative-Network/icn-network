#!/bin/bash
# ICN Network - Storage Encryption Demo Script
# This script demonstrates the enhanced encryption features of the ICN storage system.

set -e  # Exit on error

# Colors for pretty output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== ICN Network - Secure Storage Demo ===${NC}"
echo -e "${BLUE}This demo will showcase the encrypted storage capabilities of ICN${NC}\n"

# Create directories for the demo
echo -e "${YELLOW}Setting up directories...${NC}"
mkdir -p demo_storage
mkdir -p demo_output
mkdir -p demo_keys

# Create test files
echo -e "${YELLOW}Creating test files...${NC}"
echo "This is a secret file with sensitive information." > demo_storage/secret.txt
echo "This is a public announcement." > demo_storage/announcement.txt

# Step 1: Initialize Storage
echo -e "\n${GREEN}Step 1: Initializing storage with encryption${NC}"
icn-cli storage init --path ./demo_storage --encrypted

# Step 2: Generate federation key
echo -e "\n${GREEN}Step 2: Generating federation encryption key${NC}"
icn-cli storage generate-key --output ./demo_storage/federation.key

# Step 3: Store encrypted files
echo -e "\n${GREEN}Step 3: Storing files with encryption${NC}"
icn-cli storage put --file demo_storage/secret.txt --key secret-doc --encrypted --federation default
icn-cli storage put --file demo_storage/announcement.txt --key public-doc --federation default

# Step 4: List stored files
echo -e "\n${GREEN}Step 4: Listing stored files${NC}"
icn-cli storage list --federation default

# Step 5: Retrieve encrypted file
echo -e "\n${GREEN}Step 5: Retrieving encrypted file${NC}"
icn-cli storage get --key secret-doc --output demo_output/retrieved_secret.txt --federation default

# Compare original and retrieved
echo -e "\n${YELLOW}Comparing original and retrieved encrypted file:${NC}"
echo -e "Original:"
cat demo_storage/secret.txt
echo -e "\nRetrieved:"
cat demo_output/retrieved_secret.txt

# Step 6: Generate asymmetric keys for users
echo -e "\n${GREEN}Step 6: Generating asymmetric key pairs for multiple users${NC}"
icn-cli storage generate-key-pair --output-dir ./demo_keys/alice
icn-cli storage generate-key-pair --output-dir ./demo_keys/bob

# Step 7: Encrypt file for specific recipients
echo -e "\n${GREEN}Step 7: Encrypting file for specific recipients${NC}"
icn-cli storage encrypt-for --input demo_storage/secret.txt --output demo_output/for_alice_and_bob.enc --recipients "./demo_keys/alice/public.key,./demo_keys/bob/public.key"

# Step 8: Alice decrypts the file
echo -e "\n${GREEN}Step 8: Alice decrypts the file with her private key${NC}"
icn-cli storage decrypt-with --input demo_output/for_alice_and_bob.enc --output demo_output/alice_decrypted.txt --private-key ./demo_keys/alice/private.key

# Step 9: Bob decrypts the file
echo -e "\n${GREEN}Step 9: Bob decrypts the file with his private key${NC}"
icn-cli storage decrypt-with --input demo_output/for_alice_and_bob.enc --output demo_output/bob_decrypted.txt --private-key ./demo_keys/bob/private.key

# Compare decrypted files
echo -e "\n${YELLOW}Verifying decrypted content:${NC}"
echo -e "Original:"
cat demo_storage/secret.txt
echo -e "\nAlice's copy:"
cat demo_output/alice_decrypted.txt
echo -e "\nBob's copy:"
cat demo_output/bob_decrypted.txt

# Step 10: Export and import federation key
echo -e "\n${GREEN}Step 10: Exporting federation key for sharing${NC}"
icn-cli storage export-key --federation default --output demo_output/shared_federation_key.json

echo -e "\n${GREEN}Step 11: Importing federation key${NC}"
# In a real scenario, this would be done on another machine
icn-cli storage import-key --federation default --key-file demo_output/shared_federation_key.json

# Step 12: View version history
echo -e "\n${GREEN}Step 12: Viewing version history${NC}"
icn-cli storage history --key secret-doc --federation default

echo -e "\n${BLUE}Demo completed successfully!${NC}"
echo -e "${BLUE}Encrypted files and keys are in the demo_output directory.${NC}"
echo -e "${YELLOW}For security, you should delete the demo keys and files when done.${NC}" 