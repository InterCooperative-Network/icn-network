#!/bin/bash

# Color codes for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}==================================================${NC}"
echo -e "${BLUE}ICN Network Identity-Integrated Storage Demo${NC}"
echo -e "${BLUE}==================================================${NC}"

# Create directories for demo
echo -e "\n${YELLOW}Creating demo directories...${NC}"
mkdir -p demo_storage
mkdir -p demo_keys
mkdir -p demo_output
mkdir -p demo_did

# Clean up any previous demo files
rm -rf demo_storage/* demo_keys/* demo_output/* demo_did/*

# Generate test files
echo -e "\n${YELLOW}Generating test files...${NC}"
echo "This is a secret document. Only authorized DIDs should access it." > secret_document.txt
echo "This is a public announcement for everyone to read." > public_announcement.txt
echo "This document contains federation policy information." > federation_policy.txt

# Initialize storage system with identity integration
echo -e "\n${YELLOW}Initializing storage system...${NC}"
icn-cli storage init --path ./demo_storage --encrypted

# Generate federation encryption key
echo -e "\n${YELLOW}Generating federation encryption key...${NC}"
icn-cli storage generate-key --output ./demo_keys/federation.key

# Create DID documents for Alice, Bob, and Charlie
echo -e "\n${YELLOW}Creating DID documents...${NC}"

cat > demo_did/alice_did.json << EOL
{
  "id": "did:icn:alice",
  "controller": "did:icn:alice",
  "verification_method": [
    {
      "id": "did:icn:alice#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:alice",
      "public_key_base58": "z6MkhaHgzg5sMnmJioDXNkZCVKTBNXbf1J96zNhCZuCmNF7q"
    }
  ],
  "authentication": [
    "did:icn:alice#key-1"
  ],
  "service": [
    {
      "id": "did:icn:alice#storage",
      "type": "ICNStorage",
      "service_endpoint": "https://storage.icn.network/alice"
    }
  ]
}
EOL

cat > demo_did/bob_did.json << EOL
{
  "id": "did:icn:bob",
  "controller": "did:icn:bob",
  "verification_method": [
    {
      "id": "did:icn:bob#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:bob",
      "public_key_base58": "z6MkhaBwTCDxUxhGiVH9JidrAQVSQ6YZUW15GrGNB36WfD7S"
    }
  ],
  "authentication": [
    "did:icn:bob#key-1"
  ],
  "service": [
    {
      "id": "did:icn:bob#storage",
      "type": "ICNStorage",
      "service_endpoint": "https://storage.icn.network/bob"
    }
  ]
}
EOL

cat > demo_did/charlie_did.json << EOL
{
  "id": "did:icn:charlie",
  "controller": "did:icn:charlie",
  "verification_method": [
    {
      "id": "did:icn:charlie#key-1",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:charlie",
      "public_key_base58": "z6MkhaWfJcoZNQSbUBgGx1NVUuLesnZFHxD2G5ARXNCgyFKT"
    }
  ],
  "authentication": [
    "did:icn:charlie#key-1"
  ],
  "service": [
    {
      "id": "did:icn:charlie#storage",
      "type": "ICNStorage",
      "service_endpoint": "https://storage.icn.network/charlie"
    }
  ]
}
EOL

# Generate key pairs for DIDs (in a real system, these would be managed by the DID holder)
echo -e "\n${YELLOW}Generating key pairs for DIDs...${NC}"
# For demo purposes, we simulate key generation 
echo "-----ALICE PRIVATE KEY-----" > demo_keys/alice.key
echo "-----BOB PRIVATE KEY-----" > demo_keys/bob.key 
echo "-----CHARLIE PRIVATE KEY-----" > demo_keys/charlie.key

# Register DIDs in the system
echo -e "\n${YELLOW}Registering DIDs in the system...${NC}"
echo -e "${GREEN}Registering Alice's DID...${NC}"
icn-cli identity-storage register-did --did "did:icn:alice" --document demo_did/alice_did.json --federation test-fed

echo -e "${GREEN}Registering Bob's DID...${NC}"
icn-cli identity-storage register-did --did "did:icn:bob" --document demo_did/bob_did.json --federation test-fed

echo -e "${GREEN}Registering Charlie's DID...${NC}"
icn-cli identity-storage register-did --did "did:icn:charlie" --document demo_did/charlie_did.json --federation test-fed

# Map DIDs to member IDs
echo -e "\n${YELLOW}Mapping DIDs to member IDs...${NC}"
icn-cli identity-storage map-did-to-member --did "did:icn:alice" --member-id "alice" --federation test-fed
icn-cli identity-storage map-did-to-member --did "did:icn:bob" --member-id "bob" --federation test-fed
icn-cli identity-storage map-did-to-member --did "did:icn:charlie" --member-id "charlie" --federation test-fed

# Create access control policies
echo -e "\n${YELLOW}Creating access control policies...${NC}"

# Create a policy file for secret documents
cat > demo_storage/secret_policy.json << EOL
[
  {
    "pattern": "secret_*",
    "member_id": "alice",
    "permissions": ["read", "write"]
  },
  {
    "pattern": "secret_*",
    "member_id": "bob",
    "permissions": ["read"]
  }
]
EOL

# Create a policy file for public documents
cat > demo_storage/public_policy.json << EOL
[
  {
    "pattern": "public_*",
    "member_id": "*",
    "permissions": ["read"]
  },
  {
    "pattern": "public_*",
    "member_id": "alice",
    "permissions": ["read", "write"]
  }
]
EOL

# Create a policy file for federation documents
cat > demo_storage/federation_policy.json << EOL
[
  {
    "pattern": "federation_*",
    "member_id": "*",
    "permissions": ["read"]
  }
]
EOL

# Apply the policies using Alice's DID
echo -e "${GREEN}Applying secret document policy (Alice)...${NC}"
icn-cli identity-storage create-access-policy --did "did:icn:alice" --challenge "timestamp=1621500000" --signature "alice_signature" --policy-file demo_storage/secret_policy.json --federation test-fed

echo -e "${GREEN}Applying public document policy (Alice)...${NC}"
icn-cli identity-storage create-access-policy --did "did:icn:alice" --challenge "timestamp=1621500001" --signature "alice_signature" --policy-file demo_storage/public_policy.json --federation test-fed

echo -e "${GREEN}Applying federation document policy (Alice)...${NC}"
icn-cli identity-storage create-access-policy --did "did:icn:alice" --challenge "timestamp=1621500002" --signature "alice_signature" --policy-file demo_storage/federation_policy.json --federation test-fed

# Store files with DID authentication
echo -e "\n${YELLOW}Storing files with DID authentication...${NC}"

echo -e "${GREEN}Alice storing a secret document...${NC}"
icn-cli identity-storage store-file --did "did:icn:alice" --challenge "timestamp=1621500003" --signature "alice_signature" --file secret_document.txt --key "secret_document.txt" --encrypted --federation test-fed

echo -e "${GREEN}Alice storing a public announcement...${NC}"
icn-cli identity-storage store-file --did "did:icn:alice" --challenge "timestamp=1621500004" --signature "alice_signature" --file public_announcement.txt --key "public_announcement.txt" --federation test-fed

echo -e "${GREEN}Bob attempting to store a secret document (should fail)...${NC}"
icn-cli identity-storage store-file --did "did:icn:bob" --challenge "timestamp=1621500005" --signature "bob_signature" --file federation_policy.txt --key "secret_document_from_bob.txt" --encrypted --federation test-fed || echo -e "${RED}Failed as expected - Bob doesn't have write permission to secret documents${NC}"

# List files accessible to each DID
echo -e "\n${YELLOW}Listing files accessible to each DID...${NC}"

echo -e "${GREEN}Files accessible to Alice:${NC}"
icn-cli identity-storage list-files --did "did:icn:alice" --challenge "timestamp=1621500006" --signature "alice_signature" --federation test-fed

echo -e "${GREEN}Files accessible to Bob:${NC}"
icn-cli identity-storage list-files --did "did:icn:bob" --challenge "timestamp=1621500007" --signature "bob_signature" --federation test-fed

echo -e "${GREEN}Files accessible to Charlie:${NC}"
icn-cli identity-storage list-files --did "did:icn:charlie" --challenge "timestamp=1621500008" --signature "charlie_signature" --federation test-fed

# Retrieve files with DID authentication
echo -e "\n${YELLOW}Retrieving files with DID authentication...${NC}"

echo -e "${GREEN}Alice retrieving the secret document:${NC}"
icn-cli identity-storage get-file --did "did:icn:alice" --challenge "timestamp=1621500009" --signature "alice_signature" --key "secret_document.txt" --output "demo_output/alice_secret.txt" --federation test-fed
echo -e "Content retrieved by Alice:"
cat demo_output/alice_secret.txt

echo -e "\n${GREEN}Bob retrieving the secret document:${NC}"
icn-cli identity-storage get-file --did "did:icn:bob" --challenge "timestamp=1621500010" --signature "bob_signature" --key "secret_document.txt" --output "demo_output/bob_secret.txt" --federation test-fed
echo -e "Content retrieved by Bob:"
cat demo_output/bob_secret.txt

echo -e "\n${GREEN}Charlie attempting to retrieve the secret document (should fail):${NC}"
icn-cli identity-storage get-file --did "did:icn:charlie" --challenge "timestamp=1621500011" --signature "charlie_signature" --key "secret_document.txt" --output "demo_output/charlie_secret.txt" --federation test-fed || echo -e "${RED}Failed as expected - Charlie doesn't have access to secret documents${NC}"

echo -e "\n${GREEN}Charlie retrieving the public announcement:${NC}"
icn-cli identity-storage get-file --did "did:icn:charlie" --challenge "timestamp=1621500012" --signature "charlie_signature" --key "public_announcement.txt" --output "demo_output/charlie_public.txt" --federation test-fed
echo -e "Content retrieved by Charlie:"
cat demo_output/charlie_public.txt

# Demonstrate policy update
echo -e "\n${YELLOW}Demonstrating policy update...${NC}"

# Create an updated policy file for secret documents - grant Charlie read access
cat > demo_storage/updated_secret_policy.json << EOL
[
  {
    "pattern": "secret_*",
    "member_id": "alice",
    "permissions": ["read", "write"]
  },
  {
    "pattern": "secret_*",
    "member_id": "bob",
    "permissions": ["read"]
  },
  {
    "pattern": "secret_*",
    "member_id": "charlie",
    "permissions": ["read"]
  }
]
EOL

echo -e "${GREEN}Alice updating the secret document policy to grant Charlie read access...${NC}"
icn-cli identity-storage create-access-policy --did "did:icn:alice" --challenge "timestamp=1621500013" --signature "alice_signature" --policy-file demo_storage/updated_secret_policy.json --federation test-fed

echo -e "\n${GREEN}Charlie now retrieving the secret document (should succeed):${NC}"
icn-cli identity-storage get-file --did "did:icn:charlie" --challenge "timestamp=1621500014" --signature "charlie_signature" --key "secret_document.txt" --output "demo_output/charlie_secret_after_policy_update.txt" --federation test-fed
echo -e "Content retrieved by Charlie after policy update:"
cat demo_output/charlie_secret_after_policy_update.txt

# Demonstrate DID rotation
echo -e "\n${YELLOW}Demonstrating DID key rotation...${NC}"

# Create new DID document for Alice with rotated key
cat > demo_did/alice_did_updated.json << EOL
{
  "id": "did:icn:alice",
  "controller": "did:icn:alice",
  "verification_method": [
    {
      "id": "did:icn:alice#key-2",
      "type": "Ed25519VerificationKey2020",
      "controller": "did:icn:alice",
      "public_key_base58": "z6MkhaYZWfCiTQJV5Vg94Lj8hSajidHYjnJRoEaZMuCqQXYe"
    }
  ],
  "authentication": [
    "did:icn:alice#key-2"
  ],
  "service": [
    {
      "id": "did:icn:alice#storage",
      "type": "ICNStorage",
      "service_endpoint": "https://storage.icn.network/alice"
    }
  ]
}
EOL

echo -e "${GREEN}Alice registering an updated DID document with a new key...${NC}"
icn-cli identity-storage register-did --did "did:icn:alice" --document demo_did/alice_did_updated.json --federation test-fed

echo -e "${GREEN}Alice storing a file with her new key...${NC}"
icn-cli identity-storage store-file --did "did:icn:alice" --challenge "timestamp=1621500015" --signature "alice_new_signature" --file federation_policy.txt --key "federation_policy.txt" --federation test-fed

# Verify federation policy is readable by all
echo -e "\n${YELLOW}Verifying federation policy is readable by all...${NC}"

echo -e "${GREEN}Bob retrieving the federation policy:${NC}"
icn-cli identity-storage get-file --did "did:icn:bob" --challenge "timestamp=1621500016" --signature "bob_signature" --key "federation_policy.txt" --output "demo_output/bob_federation_policy.txt" --federation test-fed
echo -e "Federation policy content retrieved by Bob:"
cat demo_output/bob_federation_policy.txt

echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Identity-Integrated Storage Demo Completed${NC}"
echo -e "${BLUE}==================================================${NC}"

echo -e "\n${YELLOW}Clean up demo files? (y/n)${NC}"
read -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Cleaning up demo files...${NC}"
    rm -rf demo_storage demo_keys demo_output demo_did
    rm -f secret_document.txt public_announcement.txt federation_policy.txt
    echo -e "${GREEN}Cleanup complete.${NC}"
fi 