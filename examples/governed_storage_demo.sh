#!/bin/bash
# ICN Network - Governance-Controlled Storage Demo Script
# This script demonstrates how storage can be governed through the policy system.

set -e  # Exit on error

# Colors for pretty output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}=== ICN Network - Governance-Controlled Storage Demo ===${NC}"
echo -e "${BLUE}This demo will showcase how storage is managed through governance policies${NC}\n"

# Create directories for the demo
echo -e "${YELLOW}Setting up directories...${NC}"
mkdir -p demo_storage
mkdir -p demo_output
mkdir -p demo_policies

# Create test files
echo -e "${YELLOW}Creating test files...${NC}"
echo "This is a confidential document" > demo_storage/confidential.txt
echo "This is a public document" > demo_storage/public.txt
echo "This is Alice's personal file" > demo_storage/alice_notes.txt
echo "This is Bob's personal file" > demo_storage/bob_notes.txt

# Step 1: Initialize Storage
echo -e "\n${GREEN}Step 1: Initializing storage with encryption${NC}"
icn-cli storage init --path ./demo_storage --encrypted

# Step 2: Create members
echo -e "\n${GREEN}Step 2: Creating member information${NC}"
# In a real system, this would involve the identity system
# For demo purposes, we'll just define member IDs
ALICE_ID="alice@example.org"
BOB_ID="bob@example.org"
ADMIN_ID="admin@example.org"

# Step 3: Create an access control policy file
echo -e "\n${GREEN}Step 3: Creating access control policy${NC}"
cat > demo_policies/access_policy.json << EOF
[
  {
    "member_id": "${ADMIN_ID}",
    "path_pattern": "*",
    "can_read": true,
    "can_write": true,
    "can_grant": true
  },
  {
    "member_id": "${ALICE_ID}",
    "path_pattern": "public*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  },
  {
    "member_id": "${ALICE_ID}",
    "path_pattern": "alice*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  },
  {
    "member_id": "${BOB_ID}",
    "path_pattern": "public*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  },
  {
    "member_id": "${BOB_ID}",
    "path_pattern": "bob*",
    "can_read": true,
    "can_write": true,
    "can_grant": false
  }
]
EOF

# Step 4: Create a storage quota policy file
echo -e "\n${GREEN}Step 4: Creating storage quota policy${NC}"
cat > demo_policies/quota_policy.json << EOF
[
  {
    "target_id": "${ALICE_ID}",
    "max_bytes": 1048576,
    "max_files": 10,
    "max_file_size": 524288
  },
  {
    "target_id": "${BOB_ID}",
    "max_bytes": 1048576,
    "max_files": 10,
    "max_file_size": 524288
  }
]
EOF

# Step 5: Create a federation-wide quota policy
echo -e "\n${GREEN}Step 5: Creating federation quota policy${NC}"
cat > demo_policies/federation_quota.json << EOF
{
  "target_id": "federation",
  "max_bytes": 10485760,
  "max_files": 100
}
EOF

# Step 6: Propose the access control policy
echo -e "\n${GREEN}Step 6: Proposing access control policy${NC}"
icn-cli governed-storage propose-policy \
  --proposer "${ADMIN_ID}" \
  --title "Basic Access Control Policy" \
  --description "Defines who can access which files" \
  --policy-type "access-control" \
  --content-file "demo_policies/access_policy.json" \
  --federation "default"

# For demo purposes, we'll assume the proposal is immediately approved
# In a real system, a voting process would take place
ACCESS_PROPOSAL_ID=$(icn-cli governance list-proposals | grep "Basic Access Control Policy" | awk '{print $1}')
echo -e "${YELLOW}Access policy proposal ID: ${ACCESS_PROPOSAL_ID}${NC}"

echo -e "\n${GREEN}Step 7: Approving and applying access control policy${NC}"
icn-cli governance update-status --id "${ACCESS_PROPOSAL_ID}" --status "approved"
icn-cli governed-storage apply-policy --proposal-id "${ACCESS_PROPOSAL_ID}"

# Step 8: Propose the quota policy
echo -e "\n${GREEN}Step 8: Proposing member quota policy${NC}"
icn-cli governed-storage propose-policy \
  --proposer "${ADMIN_ID}" \
  --title "Member Storage Quotas" \
  --description "Defines storage limits for members" \
  --policy-type "member-quota" \
  --content-file "demo_policies/quota_policy.json" \
  --federation "default"

# For demo purposes, we'll assume the proposal is immediately approved
QUOTA_PROPOSAL_ID=$(icn-cli governance list-proposals | grep "Member Storage Quotas" | awk '{print $1}')
echo -e "${YELLOW}Quota policy proposal ID: ${QUOTA_PROPOSAL_ID}${NC}"

echo -e "\n${GREEN}Step 9: Approving and applying quota policy${NC}"
icn-cli governance update-status --id "${QUOTA_PROPOSAL_ID}" --status "approved"
icn-cli governed-storage apply-policy --proposal-id "${QUOTA_PROPOSAL_ID}"

# Step 10: Propose the federation quota policy
echo -e "\n${GREEN}Step 10: Proposing federation quota policy${NC}"
icn-cli governed-storage propose-policy \
  --proposer "${ADMIN_ID}" \
  --title "Federation Storage Quota" \
  --description "Defines overall storage limits for the federation" \
  --policy-type "federation-quota" \
  --content-file "demo_policies/federation_quota.json" \
  --federation "default"

# For demo purposes, we'll assume the proposal is immediately approved
FED_QUOTA_PROPOSAL_ID=$(icn-cli governance list-proposals | grep "Federation Storage Quota" | awk '{print $1}')
echo -e "${YELLOW}Federation quota policy proposal ID: ${FED_QUOTA_PROPOSAL_ID}${NC}"

echo -e "\n${GREEN}Step 11: Approving and applying federation quota policy${NC}"
icn-cli governance update-status --id "${FED_QUOTA_PROPOSAL_ID}" --status "approved"
icn-cli governed-storage apply-policy --proposal-id "${FED_QUOTA_PROPOSAL_ID}"

# Step 12: List active storage policies
echo -e "\n${GREEN}Step 12: Listing active storage policies${NC}"
icn-cli governed-storage list-policies

# Step 13: Store files with governance checks
echo -e "\n${GREEN}Step 13: Storing files as different members${NC}"
echo -e "${YELLOW}Alice stores a public document${NC}"
icn-cli governed-storage store-file \
  --file "demo_storage/public.txt" \
  --key "public-doc" \
  --member "${ALICE_ID}" \
  --federation "default"

echo -e "${YELLOW}Alice stores her personal file${NC}"
icn-cli governed-storage store-file \
  --file "demo_storage/alice_notes.txt" \
  --key "alice-notes" \
  --member "${ALICE_ID}" \
  --federation "default"

echo -e "${YELLOW}Bob stores his personal file${NC}"
icn-cli governed-storage store-file \
  --file "demo_storage/bob_notes.txt" \
  --key "bob-notes" \
  --member "${BOB_ID}" \
  --federation "default"

echo -e "${YELLOW}Admin stores a confidential document${NC}"
icn-cli governed-storage store-file \
  --file "demo_storage/confidential.txt" \
  --key "confidential-doc" \
  --member "${ADMIN_ID}" \
  --encrypted \
  --federation "default"

# Step 14: Try access violations
echo -e "\n${GREEN}Step 14: Testing access control enforcement${NC}"

echo -e "${YELLOW}Alice tries to access Bob's file (should fail)${NC}"
if icn-cli governed-storage get-file \
  --key "bob-notes" \
  --member "${ALICE_ID}" \
  --output "demo_output/alice_accessed_bob.txt" \
  --federation "default" 2>/dev/null; then
  echo -e "${RED}Failed: Alice was able to access Bob's file!${NC}"
else
  echo -e "${GREEN}Success: Access control prevented Alice from accessing Bob's file${NC}"
fi

echo -e "${YELLOW}Bob tries to access the confidential document (should fail)${NC}"
if icn-cli governed-storage get-file \
  --key "confidential-doc" \
  --member "${BOB_ID}" \
  --output "demo_output/bob_accessed_confidential.txt" \
  --federation "default" 2>/dev/null; then
  echo -e "${RED}Failed: Bob was able to access confidential document!${NC}"
else
  echo -e "${GREEN}Success: Access control prevented Bob from accessing confidential document${NC}"
fi

# Step 15: List files accessible by each member
echo -e "\n${GREEN}Step 15: Listing files accessible by each member${NC}"

echo -e "${YELLOW}Files accessible by Alice:${NC}"
icn-cli governed-storage list-files --member "${ALICE_ID}" --federation "default"

echo -e "\n${YELLOW}Files accessible by Bob:${NC}"
icn-cli governed-storage list-files --member "${BOB_ID}" --federation "default"

echo -e "\n${YELLOW}Files accessible by Admin:${NC}"
icn-cli governed-storage list-files --member "${ADMIN_ID}" --federation "default"

# Step 16: Show policy schemas
echo -e "\n${GREEN}Step 16: Showing policy JSON schemas${NC}"
echo -e "${YELLOW}Federation quota schema:${NC}"
icn-cli governed-storage show-schema --policy-type "federation-quota"

echo -e "\n${YELLOW}Access control schema:${NC}"
icn-cli governed-storage show-schema --policy-type "access-control"

echo -e "\n${BLUE}Demo completed successfully!${NC}"
echo -e "${BLUE}This demo showed how governance-based policies control storage access.${NC}"
echo -e "${YELLOW}In a production environment, these policies would be voted on by federation members.${NC}" 