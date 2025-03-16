#!/bin/bash
# Setup environment for ICN Network
# This script checks prerequisites and prepares the environment for ICN

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print header
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}ICN Environment Setup Script${NC}"
echo -e "${BLUE}======================================${NC}"

# Create data directories
echo -e "${YELLOW}Creating data directories...${NC}"
mkdir -p ./data/icn/{identity,ledger,governance,network,storage} ./data/log/icn
echo -e "${GREEN}Data directories created${NC}"

# Check for WSL
if grep -q Microsoft /proc/version || grep -q microsoft /proc/version; then
    echo -e "${YELLOW}Running in Windows Subsystem for Linux (WSL)${NC}"
    WSL=true
else
    echo -e "${GREEN}Running on native Linux${NC}"
    WSL=false
fi

# Check for required commands
echo -e "${YELLOW}Checking required commands...${NC}"
MISSING_COMMANDS=false

for cmd in cargo jq openssl ip; do
    if ! command -v $cmd &> /dev/null; then
        echo -e "${RED}Error: $cmd command not found. Please install it before continuing.${NC}"
        MISSING_COMMANDS=true
    else
        echo -e "${GREEN}Found command: $cmd${NC}"
    fi
done

if $MISSING_COMMANDS; then
    echo -e "${YELLOW}Installing missing dependencies would require:${NC}"
    echo -e "${YELLOW}sudo apt update${NC}"
    echo -e "${YELLOW}sudo apt install build-essential pkg-config libssl-dev jq iproute2${NC}"
    echo -e "${YELLOW}For Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
fi

# Check IPv6 support
echo -e "${YELLOW}Checking IPv6 support...${NC}"
if ip -6 addr show | grep -q "scope global"; then
    echo -e "${GREEN}Found global IPv6 address${NC}"
    IPV6_GLOBAL=true
else
    echo -e "${YELLOW}No global IPv6 address found. Using local addresses (::1) for testing.${NC}"
    IPV6_GLOBAL=false
fi

# Check for loopback IPv6
if ip -6 addr show | grep -q "::1"; then
    echo -e "${GREEN}IPv6 loopback interface is available${NC}"
    IPV6_LOOPBACK=true
else
    echo -e "${RED}IPv6 loopback interface not found. IPv6 testing may not work.${NC}"
    IPV6_LOOPBACK=false
fi

if $WSL; then
    echo -e "${YELLOW}WSL-specific checks:${NC}"
    
    # Check WSL version
    if [ -f /proc/sys/kernel/osrelease ]; then
        WSL_VERSION=$(grep -o "Microsoft-[a-zA-Z0-9]*" /proc/sys/kernel/osrelease || echo "Unknown")
        echo -e "${GREEN}WSL Version: $WSL_VERSION${NC}"
    fi
    
    # Check terminal type
    if [ "$TERM_PROGRAM" = "vscode" ]; then
        echo -e "${YELLOW}Running in VS Code terminal${NC}"
        echo -e "${YELLOW}Tip: Make sure you're using bash, not PowerShell in VS Code${NC}"
        echo -e "${YELLOW}    You can switch using the terminal dropdown or by typing 'bash'${NC}"
    fi
    
    # Check shell
    if [[ "$SHELL" == *"bash"* ]]; then
        echo -e "${GREEN}Using Bash shell${NC}"
    else
        echo -e "${YELLOW}Not using Bash shell. Some scripts may not work correctly.${NC}"
        echo -e "${YELLOW}Run 'bash' to switch to Bash shell${NC}"
    fi
fi

# Check for terminal support
echo -e "${YELLOW}Checking terminal support for node visualization...${NC}"
TERMINAL=""
if command -v gnome-terminal &> /dev/null; then
    TERMINAL="gnome-terminal"
    echo -e "${GREEN}Found gnome-terminal${NC}"
elif command -v xterm &> /dev/null; then
    TERMINAL="xterm"
    echo -e "${GREEN}Found xterm${NC}"
elif command -v konsole &> /dev/null; then
    TERMINAL="konsole"
    echo -e "${GREEN}Found konsole${NC}"
else
    echo -e "${YELLOW}No graphical terminal found. Nodes will run in background.${NC}"
    if $WSL; then
        echo -e "${YELLOW}WSL tip: Install Windows Terminal for better experience${NC}"
    fi
fi

# Make scripts executable
echo -e "${YELLOW}Making scripts executable...${NC}"
chmod +x scripts/*.sh
if [ -f ./start_network.sh ]; then
    chmod +x ./start_network.sh
fi
if [ -f ./start_ipv6_network.sh ]; then
    chmod +x ./start_ipv6_network.sh
fi
echo -e "${GREEN}Scripts are now executable${NC}"

# Network selection help
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Environment setup complete!${NC}"
echo -e "${GREEN}To bootstrap a standard network:${NC}"
echo -e "${GREEN}  bash scripts/bootstrap_network.sh icn-testnet 3${NC}"
echo -e "${GREEN}To bootstrap an IPv6 network:${NC}"
echo -e "${GREEN}  bash scripts/bootstrap_ipv6_network.sh icn-testnet-ipv6 3${NC}"

if $IPV6_GLOBAL; then
    echo -e "${GREEN}Your system has global IPv6 addresses available.${NC}"
    echo -e "${GREEN}You can use either network type.${NC}"
elif $IPV6_LOOPBACK; then
    echo -e "${YELLOW}Your system only has IPv6 loopback available.${NC}"
    echo -e "${YELLOW}IPv6 network will only work locally.${NC}"
else
    echo -e "${RED}Your system does not appear to have IPv6 support.${NC}"
    echo -e "${RED}Use the standard network instead.${NC}"
fi

echo -e "${BLUE}======================================${NC}" 