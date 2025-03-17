#!/bin/bash

# Color codes for better readability
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}==================================================${NC}"
echo -e "${BLUE}ICN Network Credential-Based Storage Demo${NC}"
echo -e "${BLUE}==================================================${NC}"

# Create directories for demo
echo -e "\n${YELLOW}Creating demo directories...${NC}"
mkdir -p demo_storage
mkdir -p demo_keys
mkdir -p demo_output
mkdir -p demo_did
mkdir -p demo_credentials

# Clean up any previous demo files
rm -rf demo_storage/* demo_keys/* demo_output/* demo_did/* demo_credentials/*

# Generate test files
echo -e "\n${YELLOW}Generating test files...${NC}"
echo "This is a confidential HR document." > hr_document.txt
echo "This is a financial report for Q2." > finance_report.txt
echo "This is a public announcement for all departments." > public_announcement.txt
echo "This is a sensitive engineering specification." > engineering_spec.txt

# Initialize storage system with credential support
echo -e "\n${YELLOW}Initializing storage system...${NC}"
icn-cli storage init --path ./demo_storage --encrypted

# Initialize credential storage
echo -e "\n${YELLOW}Initializing credential storage...${NC}"
icn-cli credential-storage init --path ./demo_storage --federation test-fed

# Generate federation encryption key
echo -e "\n${YELLOW}Generating federation encryption key...${NC}"
icn-cli storage generate-key --output ./demo_keys/federation.key

# Create DID documents for our users
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

# Create verifiable credentials
echo -e "\n${YELLOW}Creating verifiable credentials...${NC}"

# HR department credential for Alice
cat > demo_credentials/alice_hr_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:1",
  "type": ["VerifiableCredential", "DepartmentCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z",
  "expirationDate": "2033-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:alice",
    "department": "HR",
    "role": "Director",
    "clearance": "level-3"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

# Finance department credential for Bob
cat > demo_credentials/bob_finance_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:2",
  "type": ["VerifiableCredential", "DepartmentCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z",
  "expirationDate": "2033-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:bob",
    "department": "Finance",
    "role": "Analyst",
    "clearance": "level-2"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

# Engineering department credential for Charlie
cat > demo_credentials/charlie_engineering_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:3",
  "type": ["VerifiableCredential", "DepartmentCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z",
  "expirationDate": "2033-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:charlie",
    "department": "Engineering",
    "role": "Lead",
    "clearance": "level-3"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

# Management credential for Alice (multi-department access)
cat > demo_credentials/alice_management_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:4",
  "type": ["VerifiableCredential", "ManagementCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2023-01-01T00:00:00Z",
  "expirationDate": "2033-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:alice",
    "position": "Executive",
    "clearance": "level-4"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2023-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

# Register credentials
echo -e "\n${YELLOW}Registering credentials...${NC}"
echo -e "${GREEN}Registering Alice's HR credential...${NC}"
icn-cli credential-storage register-credential --credential demo_credentials/alice_hr_credential.json --federation test-fed

echo -e "${GREEN}Registering Bob's Finance credential...${NC}"
icn-cli credential-storage register-credential --credential demo_credentials/bob_finance_credential.json --federation test-fed

echo -e "${GREEN}Registering Charlie's Engineering credential...${NC}"
icn-cli credential-storage register-credential --credential demo_credentials/charlie_engineering_credential.json --federation test-fed

echo -e "${GREEN}Registering Alice's Management credential...${NC}"
icn-cli credential-storage register-credential --credential demo_credentials/alice_management_credential.json --federation test-fed

# Create credential-based access rules
echo -e "\n${YELLOW}Creating credential-based access rules...${NC}"

echo -e "${GREEN}Creating HR department access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500000" \
  --signature "alice_signature" \
  --pattern "hr_*" \
  --credential-types "DepartmentCredential" \
  --attributes '{"department": "HR"}' \
  --permissions "read,write" \
  --federation test-fed

echo -e "${GREEN}Creating Finance department access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500001" \
  --signature "alice_signature" \
  --pattern "finance_*" \
  --credential-types "DepartmentCredential" \
  --attributes '{"department": "Finance"}' \
  --permissions "read,write" \
  --federation test-fed

echo -e "${GREEN}Creating Engineering department access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500002" \
  --signature "alice_signature" \
  --pattern "engineering_*" \
  --credential-types "DepartmentCredential" \
  --attributes '{"department": "Engineering"}' \
  --permissions "read,write" \
  --federation test-fed

echo -e "${GREEN}Creating high clearance access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500003" \
  --signature "alice_signature" \
  --pattern "*" \
  --credential-types "ManagementCredential" \
  --attributes '{"clearance": "level-4"}' \
  --permissions "read" \
  --federation test-fed

echo -e "${GREEN}Creating public access rule...${NC}"
icn-cli credential-storage create-access-rule \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500004" \
  --signature "alice_signature" \
  --pattern "public_*" \
  --credential-types "DepartmentCredential" \
  --attributes '{}' \
  --permissions "read" \
  --federation test-fed

# Store files with DID and credential authentication
echo -e "\n${YELLOW}Storing files with credential-based authentication...${NC}"

echo -e "${GREEN}Alice storing an HR document...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500010" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --file hr_document.txt \
  --key "hr_document.txt" \
  --encrypted \
  --federation test-fed

echo -e "${GREEN}Bob storing a Finance report...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:bob" \
  --challenge "timestamp=1621500011" \
  --signature "bob_signature" \
  --credential-id "credential:2" \
  --file finance_report.txt \
  --key "finance_report.txt" \
  --encrypted \
  --federation test-fed

echo -e "${GREEN}Charlie storing an Engineering specification...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:charlie" \
  --challenge "timestamp=1621500012" \
  --signature "charlie_signature" \
  --credential-id "credential:3" \
  --file engineering_spec.txt \
  --key "engineering_spec.txt" \
  --encrypted \
  --federation test-fed

echo -e "${GREEN}Alice storing a public announcement...${NC}"
icn-cli credential-storage store-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500013" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --file public_announcement.txt \
  --key "public_announcement.txt" \
  --federation test-fed

# List files accessible to each user with their credentials
echo -e "\n${YELLOW}Listing files accessible with credentials...${NC}"

echo -e "${GREEN}Files accessible to Alice with HR credential:${NC}"
icn-cli credential-storage list-files \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500020" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --federation test-fed

echo -e "${GREEN}Files accessible to Bob with Finance credential:${NC}"
icn-cli credential-storage list-files \
  --did "did:icn:bob" \
  --challenge "timestamp=1621500021" \
  --signature "bob_signature" \
  --credential-id "credential:2" \
  --federation test-fed

echo -e "${GREEN}Files accessible to Charlie with Engineering credential:${NC}"
icn-cli credential-storage list-files \
  --did "did:icn:charlie" \
  --challenge "timestamp=1621500022" \
  --signature "charlie_signature" \
  --credential-id "credential:3" \
  --federation test-fed

echo -e "${GREEN}Files accessible to Alice with Management credential (cross-department):${NC}"
icn-cli credential-storage list-files \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500023" \
  --signature "alice_signature" \
  --credential-id "credential:4" \
  --federation test-fed

# Retrieve files with credential-based authentication
echo -e "\n${YELLOW}Retrieving files with credential-based authentication...${NC}"

echo -e "${GREEN}Alice retrieving the HR document with HR credential:${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500030" \
  --signature "alice_signature" \
  --credential-id "credential:1" \
  --key "hr_document.txt" \
  --output "demo_output/alice_hr_doc.txt" \
  --federation test-fed

echo -e "${GREEN}Bob attempting to retrieve the HR document with Finance credential (should fail):${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:bob" \
  --challenge "timestamp=1621500031" \
  --signature "bob_signature" \
  --credential-id "credential:2" \
  --key "hr_document.txt" \
  --output "demo_output/bob_hr_doc.txt" \
  --federation test-fed || echo -e "${RED}Failed as expected - Bob's Finance credential doesn't grant access to HR documents${NC}"

echo -e "${GREEN}Alice retrieving the Finance report with Management credential:${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:alice" \
  --challenge "timestamp=1621500032" \
  --signature "alice_signature" \
  --credential-id "credential:4" \
  --key "finance_report.txt" \
  --output "demo_output/alice_finance_report.txt" \
  --federation test-fed

echo -e "${GREEN}Charlie retrieving the public announcement with Engineering credential:${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:charlie" \
  --challenge "timestamp=1621500033" \
  --signature "charlie_signature" \
  --credential-id "credential:3" \
  --key "public_announcement.txt" \
  --output "demo_output/charlie_announcement.txt" \
  --federation test-fed

# Demonstrate credential verification
echo -e "\n${YELLOW}Demonstrating credential verification...${NC}"

echo -e "${GREEN}Verifying Alice's HR credential:${NC}"
icn-cli credential-storage verify-credential \
  --credential-id "credential:1" \
  --federation test-fed

# Save and load access rules
echo -e "\n${YELLOW}Demonstrating access rules persistence...${NC}"

echo -e "${GREEN}Saving credential access rules to file...${NC}"
icn-cli credential-storage save-access-rules \
  --output "./demo_storage/access_rules.json" \
  --federation test-fed

echo -e "${GREEN}Loading credential access rules from file...${NC}"
icn-cli credential-storage load-access-rules \
  --input "./demo_storage/access_rules.json" \
  --federation test-fed

# Demonstrate expired credential
echo -e "\n${YELLOW}Demonstrating expired credential handling...${NC}"

# Create an expired credential
cat > demo_credentials/expired_credential.json << EOL
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://www.w3.org/2018/credentials/examples/v1"
  ],
  "id": "credential:expired",
  "type": ["VerifiableCredential", "DepartmentCredential"],
  "issuer": "did:icn:issuer",
  "issuanceDate": "2020-01-01T00:00:00Z",
  "expirationDate": "2021-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:icn:bob",
    "department": "HR",
    "role": "Temporary"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2020-01-01T00:00:00Z",
    "verificationMethod": "did:icn:issuer#key-1",
    "proofPurpose": "assertionMethod",
    "jws": "eyJhbGciOiJFZERTQSIsImI2NCI6ZmFsc2UsImNyaXQiOlsiYjY0Il19..YtqjEYnFENT7fNW-COD0HAACxeuQxPKAmp4nIl8jYyO8DRb9F2ZrX1jfISf6LlKTaAECtQeJsF0k0rZVEoieCw"
  }
}
EOL

echo -e "${GREEN}Registering expired credential:${NC}"
icn-cli credential-storage register-credential \
  --credential demo_credentials/expired_credential.json \
  --federation test-fed

echo -e "${GREEN}Verifying expired credential:${NC}"
icn-cli credential-storage verify-credential \
  --credential-id "credential:expired" \
  --federation test-fed

echo -e "${GREEN}Bob attempting to access HR document with expired credential (should fail):${NC}"
icn-cli credential-storage get-file \
  --did "did:icn:bob" \
  --challenge "timestamp=1621500040" \
  --signature "bob_signature" \
  --credential-id "credential:expired" \
  --key "hr_document.txt" \
  --output "demo_output/bob_hr_doc_expired.txt" \
  --federation test-fed || echo -e "${RED}Failed as expected - Credential is expired${NC}"

echo -e "\n${BLUE}==================================================${NC}"
echo -e "${BLUE}Credential-Based Storage Demo Completed${NC}"
echo -e "${BLUE}==================================================${NC}"

echo -e "\n${YELLOW}Clean up demo files? (y/n)${NC}"
read -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${RED}Cleaning up demo files...${NC}"
    rm -rf demo_storage demo_keys demo_output demo_did demo_credentials
    rm -f hr_document.txt finance_report.txt public_announcement.txt engineering_spec.txt
    echo -e "${GREEN}Cleanup complete.${NC}"
fi 