# ICN Linux Integration Guide: Active Directory Alternative

This guide explains how to use the ICN Network as a comprehensive alternative to Active Directory for Linux-based cooperative environments.

## Overview

The ICN Network provides a decentralized alternative to corporate IT infrastructure, specifically designed for Linux environments. This guide focuses on how ICN replaces traditional Active Directory functionality:

1. **User Authentication & Authorization**
2. **Group Policy Management**
3. **Resource Access Control**
4. **System Configuration**
5. **Team Collaboration**

## System Architecture

```
┌────────────────────────────────────────────────────────┐
│                      Linux System                      │
│                                                        │
│  ┌──────────┐  ┌───────────┐  ┌────────────────────┐  │
│  │ PAM      │  │ LDAP      │  │ System             │  │
│  │ Module   │  │ Bridge    │  │ Configuration      │  │
│  └─────┬────┘  └─────┬─────┘  └──────────┬─────────┘  │
│        │            │                    │            │
└────────┼────────────┼────────────────────┼────────────┘
         │            │                    │             
         │            │                    │             
┌────────┼────────────┼────────────────────┼────────────┐
│        ▼            ▼                    ▼            │
│  ┌──────────┐  ┌───────────┐  ┌────────────────────┐  │
│  │ Identity │  │ Directory │  │ Governance         │  │
│  │ System   │  │ Service   │  │ System             │  │
│  └─────┬────┘  └─────┬─────┘  └──────────┬─────────┘  │
│        │            │                    │            │
│        └────────────┼────────────────────┘            │
│                     │                                  │
│                     ▼                                  │
│               ┌───────────┐                           │
│               │ ICN Node  │                           │
│               └───────────┘                           │
│                                                       │
└───────────────────────────────────────────────────────┘
```

## 1. User Authentication

### 1.1 PAM Integration

The ICN Identity system integrates with Linux PAM (Pluggable Authentication Modules) to enable DID-based authentication:

```bash
# /etc/pam.d/icn-auth configuration
auth     required     pam_icn.so
account  required     pam_icn.so
session  optional     pam_icn.so
```

### 1.2 Setting Up the ICN PAM Module

```bash
# Install the ICN PAM module
sudo cp /path/to/pam_icn.so /lib/security/

# Configure the ICN auth daemon
sudo cp /path/to/icn-auth-daemon.service /etc/systemd/system/
sudo systemctl enable icn-auth-daemon
sudo systemctl start icn-auth-daemon
```

### 1.3 Authentication Flow

1. User attempts to log in to Linux system
2. PAM module intercepts authentication request
3. Request is forwarded to ICN auth daemon
4. Daemon creates authentication challenge using DID verification
5. User signs challenge with their private key (via agent or security key)
6. ICN verifies signature and grants access if valid

## 2. Directory Services (LDAP Alternative)

### 2.1 ICN Directory Service

ICN provides a directory service that's compatible with LDAP clients but uses DIDs as the backend:

```bash
# Install the ICN LDAP bridge
sudo cp /path/to/icn-ldap-bridge /usr/local/bin/
sudo cp /path/to/icn-ldap-bridge.service /etc/systemd/system/
sudo systemctl enable icn-ldap-bridge
sudo systemctl start icn-ldap-bridge
```

### 2.2 Directory Configuration

Edit the ICN directory configuration in `/etc/icn/directory.yaml`:

```yaml
bind_address: 0.0.0.0
port: 389
base_dn: "dc=icn,dc=coop"
admin_did: "did:icn:coop1:admin"
groups_ou: "ou=groups"
users_ou: "ou=users"
```

### 2.3 User Management

Users are managed through ICN DIDs and automatically reflected in the directory service:

```bash
# Create a new user
icn-cli identity create-did --coop-id coop1 --name alice --role user

# Add user to group
icn-cli identity add-group --did did:icn:coop1:alice --group developers
```

### 2.4 Group Management

Groups in ICN are represented by verifiable credentials and mapped to LDAP groups:

```bash
# Create a new group
icn-cli identity create-group --coop-id coop1 --name developers

# List groups
icn-cli identity list-groups --coop-id coop1
```

## 3. Linux System Configuration

### 3.1 System Configuration Management

ICN can manage system configurations across multiple machines through the governance system:

```bash
# Create a system configuration policy
icn-cli governance create-policy --name system-config --policy-file /path/to/config-policy.json

# Apply the policy to a specific group
icn-cli governance apply-policy --name system-config --target-group developers
```

### 3.2 System Configuration Agent

Each Linux machine runs an ICN configuration agent that applies policies from the governance system:

```bash
# Install the ICN config agent
sudo cp /path/to/icn-config-agent /usr/local/bin/
sudo cp /path/to/icn-config-agent.service /etc/systemd/system/
sudo systemctl enable icn-config-agent
sudo systemctl start icn-config-agent
```

### 3.3 Configuration Policies

Example of a system configuration policy:

```json
{
  "policy_type": "system_config",
  "name": "security_hardening",
  "description": "Security hardening for developer machines",
  "config_items": [
    {
      "type": "file",
      "path": "/etc/ssh/sshd_config",
      "content_template": "PermitRootLogin no\nPasswordAuthentication no\n..."
    },
    {
      "type": "package",
      "action": "install",
      "packages": ["ufw", "fail2ban"]
    },
    {
      "type": "service",
      "name": "ufw",
      "action": "enable",
      "start": true
    }
  ]
}
```

## 4. Team Collaboration Platform

### 4.1 Setting Up the Collaboration Server

The ICN Team Collaboration Platform provides messaging, file sharing, and project management:

```bash
# Install the ICN collaboration server
sudo cp /path/to/icn-collaboration /usr/local/bin/
sudo cp /path/to/icn-collaboration.service /etc/systemd/system/
sudo systemctl enable icn-collaboration
sudo systemctl start icn-collaboration
```

### 4.2 Collaboration Configuration

Configure the collaboration platform in `/etc/icn/collaboration.yaml`:

```yaml
server:
  bind_address: 0.0.0.0
  port: 8080
  federation_id: "coop-federation-1"

storage:
  path: "/var/lib/icn/collaboration"
  max_file_size: 100MB

channels:
  - name: "general"
    description: "General discussion"
  - name: "tech"
    description: "Technical discussion"
```

### 4.3 Client Installation

Users can connect to the collaboration platform using the ICN client:

```bash
# Install the ICN client
sudo apt install icn-client

# Configure the client
icn-client config --server icn.coop.local:8080 --identity did:icn:coop1:alice
```

## 5. Resource Management

### 5.1 Resource Access Control

ICN provides fine-grained access control for cooperative resources:

```bash
# Grant access to a resource
icn-cli resource grant --resource shared-storage --user did:icn:coop1:alice --permissions read,write

# Create a shared resource
icn-cli resource create --name project-files --type storage --path /shared/projects
```

### 5.2 Resource Sharing Across Cooperatives

Resources can be shared across cooperative boundaries through federation:

```bash
# Share a resource with another cooperative
icn-cli resource share --resource project-files --with-coop coop2 --permissions read
```

## 6. Deployment Examples

### 6.1 Small Cooperative Setup

For a small cooperative (5-20 users):

```bash
# Install ICN node
sudo apt install icn-node icn-auth icn-ldap-bridge icn-collaboration

# Configure as primary node
sudo icn-node setup --type primary --coop-id coop1 --admin-did did:icn:coop1:admin

# Enable all services
sudo systemctl enable icn-node icn-auth-daemon icn-ldap-bridge icn-collaboration
sudo systemctl start icn-node icn-auth-daemon icn-ldap-bridge icn-collaboration
```

### 6.2 Federation Setup

For a federation of cooperatives:

```bash
# On primary cooperative node
sudo icn-node setup --type federation-primary --coop-id coop1 --federation-id fed1

# On secondary cooperative node
sudo icn-node setup --type federation-member --coop-id coop2 --federation-id fed1 --connect-to node1.coop1.icn
```

## 7. Client Configuration

### 7.1 User Onboarding

The process for onboarding a new user:

```bash
# Generate user DID
icn-cli identity create-did --coop-id coop1 --name bob --role user

# Generate authentication key
icn-cli identity create-key --did did:icn:coop1:bob --purpose authentication

# Create Linux user
sudo useradd -m bob

# Link Linux user to DID
sudo icn-auth link-user --system-user bob --did did:icn:coop1:bob

# Add to groups
icn-cli identity add-group --did did:icn:coop1:bob --group developers
```

### 7.2 WebAuthn Integration

For enhanced security, ICN supports WebAuthn/FIDO2 security keys:

```bash
# Register a security key
icn-cli identity register-security-key --did did:icn:coop1:bob

# Configure PAM to use security key
sudo sed -i 's/pam_icn.so/pam_icn.so webauthn/g' /etc/pam.d/icn-auth
```

## 8. Security Considerations

### 8.1 Key Management

Proper key management is essential for secure ICN deployment:

```bash
# Backup DIDs and keys
icn-cli identity backup --coop-id coop1 --output /secure/backup/coop1-keys.enc

# Rotate authentication keys
icn-cli identity rotate-key --did did:icn:coop1:bob --purpose authentication

# Set up key recovery
icn-cli identity set-recovery --did did:icn:coop1:bob --recovery-method threshold --trustees 3 --threshold 2
```

### 8.2 System Hardening

ICN deployments should follow security best practices:

1. Keep all ICN components updated to the latest version
2. Use firewalls to restrict access to ICN services
3. Enable encrypted storage for ICN data
4. Implement regular backup procedures
5. Use the WireGuard overlay network for secure communications

## 9. Monitoring and Management

### 9.1 System Status

Monitor the health of ICN services:

```bash
# Check status of all ICN services
sudo systemctl status icn-*

# View ICN logs
sudo journalctl -u icn-node -f
```

### 9.2 Management Dashboard

ICN provides a web-based management dashboard:

```bash
# Install the management dashboard
sudo apt install icn-dashboard

# Access the dashboard
# Open http://localhost:8090 in browser
```

## 10. Troubleshooting

### 10.1 Authentication Issues

If users cannot authenticate:

1. Check the ICN auth daemon is running
2. Verify user DID exists and is properly configured
3. Check PAM configuration in `/etc/pam.d/icn-auth`
4. Review auth daemon logs: `journalctl -u icn-auth-daemon -n 50`

### 10.2 Directory Service Issues

If the directory service is not working:

1. Ensure the LDAP bridge is running
2. Check network connectivity to the LDAP port
3. Verify the directory configuration in `/etc/icn/directory.yaml`
4. Test the LDAP connection: `ldapsearch -H ldap://localhost -x -b "dc=icn,dc=coop"`

## Conclusion

By following this guide, cooperatives can establish a complete Linux-based IT infrastructure that provides alternatives to corporate IT systems like Active Directory. The ICN Network delivers essential IT backbone services with added benefits of decentralization, democratic governance, and economic integration.

Future enhancements will include:
1. Enhanced group policy features
2. More application integrations
3. Expanded resource management capabilities
4. Improved security and audit features 