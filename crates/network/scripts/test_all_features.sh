#!/bin/bash
# Comprehensive test script for ICN Network features

set -e # Exit on error

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

print_header() {
    echo -e "\n${BLUE}====================================================${NC}"
    echo -e "${GREEN}$1${NC}"
    echo -e "${BLUE}====================================================${NC}\n"
}

print_step() {
    echo -e "${YELLOW}➤ $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

check_prereqs() {
    print_step "Checking prerequisites..."
    
    # Check if cargo is available
    if ! command -v cargo &> /dev/null; then
        print_error "cargo could not be found. Please install Rust."
        exit 1
    fi
    
    # Check if curl is available
    if ! command -v curl &> /dev/null; then
        print_error "curl could not be found. Please install curl."
        exit 1
    fi
    
    # Check if jq is available (for parsing JSON responses)
    if ! command -v jq &> /dev/null; then
        print_error "jq could not be found. Please install jq."
        exit 1
    fi
    
    print_success "All prerequisites met"
}

build_examples() {
    print_step "Building all examples..."
    cargo build --examples
    print_success "Build completed"
}

# Start a metrics server and test if it's accessible
test_metrics() {
    print_header "Testing Metrics"
    
    local metrics_port=9091
    print_step "Starting metrics demo on port $metrics_port..."
    
    # Start the metrics demo in the background
    cargo run --example metrics_demo -- --metrics-address "127.0.0.1:$metrics_port" &
    METRICS_PID=$!
    
    # Wait for the server to start
    print_step "Waiting for metrics server to start..."
    sleep 5
    
    # Test if the metrics endpoint is accessible
    print_step "Testing metrics endpoint..."
    if curl -s "http://127.0.0.1:$metrics_port/metrics" | grep -q "network_peers_connected"; then
        print_success "Metrics server is accessible and reporting data"
    else
        print_error "Could not access metrics or data not found"
    fi
    
    # Cleanup
    print_step "Stopping metrics demo..."
    kill $METRICS_PID
    wait $METRICS_PID 2>/dev/null || true
    print_success "Metrics test completed"
}

# Test the reputation system with sample data
test_reputation() {
    print_header "Testing Reputation System"
    
    local metrics_port=9092
    print_step "Starting reputation demo on port $metrics_port..."
    
    # Start the reputation demo in the background
    cargo run --example reputation_demo -- --metrics-address "127.0.0.1:$metrics_port" &
    REPUTATION_PID=$!
    
    # Wait for the server to start
    print_step "Waiting for reputation demo to initialize..."
    sleep 5
    
    # Test if reputation metrics are available
    print_step "Testing reputation metrics..."
    if curl -s "http://127.0.0.1:$metrics_port/metrics" | grep -q "network_reputation"; then
        print_success "Reputation metrics found"
    else
        print_error "Reputation metrics not found"
    fi
    
    # Cleanup
    print_step "Stopping reputation demo..."
    kill $REPUTATION_PID
    wait $REPUTATION_PID 2>/dev/null || true
    print_success "Reputation test completed"
}

# Test priority message processing
test_priority_messaging() {
    print_header "Testing Priority Message Processing"
    
    local metrics_port=9093
    print_step "Starting priority messaging demo on port $metrics_port..."
    
    # Start the priority messaging demo in the background
    cargo run --example priority_messaging -- --metrics-address "127.0.0.1:$metrics_port" &
    PRIORITY_PID=$!
    
    # Wait for the server to start
    print_step "Waiting for priority messaging demo to initialize..."
    sleep 8
    
    # Test if priority queue metrics are available
    print_step "Testing priority queue metrics..."
    if curl -s "http://127.0.0.1:$metrics_port/metrics" | grep -q "network_queue"; then
        print_success "Priority queue metrics found"
    else
        print_error "Priority queue metrics not found"
    fi
    
    # Cleanup
    print_step "Stopping priority messaging demo..."
    kill $PRIORITY_PID
    wait $PRIORITY_PID 2>/dev/null || true
    print_success "Priority messaging test completed"
}

# Test circuit relay functionality with a simple setup
test_circuit_relay() {
    print_header "Testing Circuit Relay"
    
    # Define ports for each node
    local relay_port=9001
    local public_port=9002
    local relay_metrics_port=9094
    local public_metrics_port=9095
    
    print_step "Starting relay server on port $relay_port..."
    cargo run --example circuit_relay_demo -- \
        relay-server --port $relay_port --metrics-address "127.0.0.1:$relay_metrics_port" &
    RELAY_PID=$!
    
    # Wait for the relay server to start
    print_step "Waiting for relay server to initialize..."
    sleep 5
    
    # Get the relay server's peer ID and multiaddress
    print_step "Getting relay server info..."
    local relay_metrics=$(curl -s "http://127.0.0.1:$relay_metrics_port/metrics")
    
    # Extract the peer ID from metrics (this is a bit of a hack, you might need to adjust)
    # In a real scenario, you might want to log this to a file or use a more reliable method
    local relay_peer_id=$(ps aux | grep "circuit_relay_demo.*relay-server" | grep -v grep | head -1)
    echo "Relay process: $relay_peer_id"
    
    # For testing purposes, we just check if the relay metrics are available
    if echo "$relay_metrics" | grep -q "network_peers_connected"; then
        print_success "Relay server metrics found"
        
        print_step "Starting public node on port $public_port..."
        # In a full test, you would connect to the relay's actual multiaddress
        # For this script, we're simplifying and just checking if the node starts
        cargo run --example circuit_relay_demo -- \
            public-node --port $public_port --relay "/ip4/127.0.0.1/tcp/$relay_port/p2p/PLACEHOLDER" \
            --metrics-address "127.0.0.1:$public_metrics_port" &
        PUBLIC_PID=$!
        
        print_step "Waiting for public node to initialize..."
        sleep 5
        
        # Check if public node metrics are available
        if curl -s "http://127.0.0.1:$public_metrics_port/metrics" | grep -q "network_peers_connected"; then
            print_success "Public node metrics found"
        else
            print_error "Public node metrics not found"
        fi
        
        # Cleanup public node
        print_step "Stopping public node..."
        kill $PUBLIC_PID
        wait $PUBLIC_PID 2>/dev/null || true
    else
        print_error "Relay server metrics not found"
    fi
    
    # Cleanup relay server
    print_step "Stopping relay server..."
    kill $RELAY_PID
    wait $RELAY_PID 2>/dev/null || true
    print_success "Circuit relay test completed"
}

# Test the integrated demo with all features
test_integrated_demo() {
    print_header "Testing Integrated Demo"
    
    local metrics_port=9096
    print_step "Starting integrated demo on port $metrics_port..."
    
    # Start the integrated demo in the background
    cargo run --example integrated_demo -- all --metrics-address "127.0.0.1:$metrics_port" &
    INTEGRATED_PID=$!
    
    # Wait for the server to start
    print_step "Waiting for integrated demo to initialize..."
    sleep 10
    
    # Test if all metrics are available
    print_step "Testing integrated metrics..."
    local metrics=$(curl -s "http://127.0.0.1:$metrics_port/metrics")
    
    local tests_passed=true
    
    # Check for metrics from each component
    if echo "$metrics" | grep -q "network_peers_connected"; then
        print_success "Connection metrics found"
    else
        print_error "Connection metrics not found"
        tests_passed=false
    fi
    
    if echo "$metrics" | grep -q "network_reputation"; then
        print_success "Reputation metrics found"
    else
        print_error "Reputation metrics not found"
        tests_passed=false
    fi
    
    if echo "$metrics" | grep -q "network_queue"; then
        print_success "Queue metrics found"
    else
        print_error "Queue metrics not found"
        tests_passed=false
    fi
    
    if echo "$metrics" | grep -q "network_relay"; then
        print_success "Relay metrics found"
    else
        print_error "Relay metrics not found"
        tests_passed=false
    fi
    
    # Cleanup
    print_step "Stopping integrated demo..."
    kill $INTEGRATED_PID
    wait $INTEGRATED_PID 2>/dev/null || true
    
    if $tests_passed; then
        print_success "Integrated demo test completed successfully"
    else
        print_error "Some integrated demo tests failed"
    fi
}

run_all_tests() {
    print_header "ICN Network Features Test Suite"
    
    local start_time=$(date +%s)
    
    check_prereqs
    build_examples
    
    test_metrics
    test_reputation
    test_priority_messaging
    test_circuit_relay
    test_integrated_demo
    
    local end_time=$(date +%s)
    local duration=$((end_time - start_time))
    
    print_header "Test Results"
    echo -e "All tests completed in ${duration} seconds"
    echo -e "\n${GREEN}You can now run the demos manually to explore each feature:${NC}"
    echo -e "  ${YELLOW}./scripts/run_demos.sh${NC}"
}

# Run all tests
run_all_tests 