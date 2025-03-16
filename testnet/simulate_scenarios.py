#!/usr/bin/env python3
"""
ICN IPv6 Overlay Network Testnet Scenario Simulator

This script runs different testing scenarios on the IPv6 overlay testnet
to verify its behavior under various conditions.
"""

import os
import re
import time
import json
import random
import signal
import argparse
import subprocess
from datetime import datetime
from typing import List, Dict, Set, Optional, Tuple

# Configuration
LOG_DIR = "./logs"
NODE_BINARY = "cargo run --bin testnet_node --"
BASE_PORT = 9000

# Regular expressions for log parsing
NODE_INITIALIZED_RE = re.compile(r"Node initialized with overlay address: (.*)")


class TestnetNode:
    """Represents a node in the testnet."""
    
    def __init__(self, node_id: str, federation: Optional[str] = None, port: int = None):
        self.node_id = node_id
        self.federation = federation
        self.port = port or (BASE_PORT + random.randint(1000, 9999))
        self.address = None
        self.process = None
        self.log_file = None
        
    def start(self, bootstrap_peers: List[str] = None, forwarding_policy: str = "ForwardKnown"):
        """Start the node process."""
        log_path = os.path.join(LOG_DIR, f"{self.node_id}.log")
        self.log_file = log_path
        
        # Build command
        cmd = [
            NODE_BINARY,
            f"--node-id={self.node_id}",
            f"--federation={self.federation or ''}",
            f"--port={self.port}",
            f"--forwarding-policy={forwarding_policy}",
            "--log-level=debug"
        ]
        
        if bootstrap_peers:
            cmd.append(f"--bootstrap-peers={','.join(bootstrap_peers)}")
        
        # Create logs directory if it doesn't exist
        os.makedirs(os.path.dirname(log_path), exist_ok=True)
        
        # Start process
        with open(log_path, "w") as log_file:
            self.process = subprocess.Popen(
                " ".join(cmd),
                stdout=log_file,
                stderr=log_file,
                shell=True,
                preexec_fn=os.setsid
            )
        
        print(f"Started node {self.node_id} (PID: {self.process.pid})")
        
        # Wait for node to initialize and get its address
        max_wait = 30  # seconds
        wait_time = 0
        sleep_interval = 0.5
        
        while wait_time < max_wait:
            if os.path.exists(log_path):
                with open(log_path, "r") as f:
                    log_content = f.read()
                    match = NODE_INITIALIZED_RE.search(log_content)
                    if match:
                        self.address = match.group(1)
                        print(f"Node {self.node_id} initialized with address: {self.address}")
                        return True
            
            time.sleep(sleep_interval)
            wait_time += sleep_interval
        
        print(f"Failed to start node {self.node_id} within {max_wait} seconds")
        return False
    
    def stop(self):
        """Stop the node process."""
        if self.process:
            try:
                os.killpg(os.getpgid(self.process.pid), signal.SIGTERM)
                print(f"Stopped node {self.node_id}")
                return True
            except Exception as e:
                print(f"Error stopping node {self.node_id}: {e}")
        return False
    
    def is_running(self):
        """Check if the node is still running."""
        if self.process:
            return self.process.poll() is None
        return False
    
    def read_logs(self):
        """Read the node's log file."""
        if self.log_file and os.path.exists(self.log_file):
            with open(self.log_file, "r") as f:
                return f.read()
        return ""


class TestnetSimulator:
    """Simulator for running test scenarios on the IPv6 overlay testnet."""
    
    def __init__(self):
        self.nodes: Dict[str, TestnetNode] = {}
        self.federations: Dict[str, List[str]] = {}
        
    def create_node(self, node_id: str, federation: Optional[str] = None) -> TestnetNode:
        """Create a new node with the given ID and federation."""
        node = TestnetNode(node_id, federation)
        self.nodes[node_id] = node
        
        # Add to federation mapping
        if federation:
            if federation not in self.federations:
                self.federations[federation] = []
            self.federations[federation].append(node_id)
        
        return node
    
    def get_federation_nodes(self, federation: str) -> List[TestnetNode]:
        """Get all nodes in the specified federation."""
        if federation not in self.federations:
            return []
        
        return [self.nodes[node_id] for node_id in self.federations[federation]]
    
    def start_node(self, node_id: str, bootstrap_peers: List[str] = None, 
                  forwarding_policy: str = "ForwardKnown") -> bool:
        """Start a node with the given parameters."""
        if node_id not in self.nodes:
            print(f"Unknown node ID: {node_id}")
            return False
        
        return self.nodes[node_id].start(bootstrap_peers, forwarding_policy)
    
    def stop_node(self, node_id: str) -> bool:
        """Stop the specified node."""
        if node_id not in self.nodes:
            print(f"Unknown node ID: {node_id}")
            return False
        
        return self.nodes[node_id].stop()
    
    def stop_all_nodes(self):
        """Stop all running nodes."""
        for node_id, node in self.nodes.items():
            if node.is_running():
                node.stop()
    
    def get_node_address(self, node_id: str) -> Optional[str]:
        """Get a node's overlay address."""
        if node_id not in self.nodes:
            print(f"Unknown node ID: {node_id}")
            return None
        
        return self.nodes[node_id].address
    
    def clean_logs(self):
        """Clean log files from previous runs."""
        if os.path.exists(LOG_DIR):
            for filename in os.listdir(LOG_DIR):
                if filename.endswith(".log"):
                    os.remove(os.path.join(LOG_DIR, filename))


def run_basic_connectivity_test(simulator: TestnetSimulator):
    """Run a basic connectivity test with nodes in each federation."""
    print("=== Running Basic Connectivity Test ===")
    
    # Clean previous logs
    simulator.clean_logs()
    
    try:
        # Create federation bootstrap nodes
        print("Creating federation bootstrap nodes...")
        fed_a_bootstrap = simulator.create_node("federation-a-bootstrap", "federation-a")
        fed_b_bootstrap = simulator.create_node("federation-b-bootstrap", "federation-b")
        
        # Start federation bootstrap nodes
        print("Starting federation bootstrap nodes...")
        simulator.start_node("federation-a-bootstrap")
        simulator.start_node("federation-b-bootstrap")
        
        # Wait for bootstrap nodes to initialize
        time.sleep(5)
        
        # Get bootstrap addresses
        fed_a_addr = simulator.get_node_address("federation-a-bootstrap")
        fed_b_addr = simulator.get_node_address("federation-b-bootstrap")
        
        if not fed_a_addr or not fed_b_addr:
            print("Failed to get bootstrap addresses")
            return False
        
        # Create member nodes
        print("Creating member nodes...")
        fed_a_node1 = simulator.create_node("federation-a-node-1", "federation-a")
        fed_a_node2 = simulator.create_node("federation-a-node-2", "federation-a")
        fed_b_node1 = simulator.create_node("federation-b-node-1", "federation-b")
        fed_b_node2 = simulator.create_node("federation-b-node-2", "federation-b")
        
        # Start member nodes
        print("Starting federation A members...")
        simulator.start_node("federation-a-node-1", [fed_a_addr])
        simulator.start_node("federation-a-node-2", [fed_a_addr])
        
        print("Starting federation B members...")
        simulator.start_node("federation-b-node-1", [fed_b_addr])
        simulator.start_node("federation-b-node-2", [fed_b_addr])
        
        # Create cross-federation node (bridge)
        print("Creating and starting cross-federation bridge node...")
        bridge_node = simulator.create_node("cross-federation-bridge", "federation-a")
        simulator.start_node("cross-federation-bridge", [fed_a_addr, fed_b_addr], "ForwardAll")
        
        # Let the network stabilize
        print("Waiting for network to stabilize...")
        time.sleep(20)
        
        # Check connectivity
        print("Checking connectivity...")
        
        # Check federation A connectivity
        fed_a_logs = simulator.nodes["federation-a-bootstrap"].read_logs()
        if "Connected to 2 peers" in fed_a_logs or "Peers: 2" in fed_a_logs:
            print("✓ Federation A bootstrap connected to federation members")
        else:
            print("✗ Federation A bootstrap not fully connected")
        
        # Check federation B connectivity
        fed_b_logs = simulator.nodes["federation-b-bootstrap"].read_logs()
        if "Connected to 2 peers" in fed_b_logs or "Peers: 2" in fed_b_logs:
            print("✓ Federation B bootstrap connected to federation members")
        else:
            print("✗ Federation B bootstrap not fully connected")
        
        # Check cross-federation bridge
        bridge_logs = simulator.nodes["cross-federation-bridge"].read_logs()
        if (("Connected to peer" in bridge_logs and "federation-a" in bridge_logs) and 
            ("Connected to peer" in bridge_logs and "federation-b" in bridge_logs)):
            print("✓ Cross-federation bridge connected to both federations")
        else:
            print("✗ Cross-federation bridge not connected to both federations")
        
        print("Basic connectivity test completed")
        return True
        
    except Exception as e:
        print(f"Error during basic connectivity test: {e}")
        return False
    finally:
        # Stop all nodes
        print("Stopping all nodes...")
        simulator.stop_all_nodes()


def run_federation_isolation_test(simulator: TestnetSimulator):
    """Test federation isolation and forwarding policies."""
    print("=== Running Federation Isolation Test ===")
    
    # Clean previous logs
    simulator.clean_logs()
    
    try:
        # Create federation bootstrap nodes
        print("Creating federation bootstrap nodes...")
        fed_a_bootstrap = simulator.create_node("federation-a-bootstrap", "federation-a")
        fed_b_bootstrap = simulator.create_node("federation-b-bootstrap", "federation-b")
        
        # Start federation bootstrap nodes
        print("Starting federation bootstrap nodes...")
        simulator.start_node("federation-a-bootstrap", forwarding_policy="NoForwarding")
        simulator.start_node("federation-b-bootstrap", forwarding_policy="NoForwarding")
        
        # Wait for bootstrap nodes to initialize
        time.sleep(5)
        
        # Get bootstrap addresses
        fed_a_addr = simulator.get_node_address("federation-a-bootstrap")
        fed_b_addr = simulator.get_node_address("federation-b-bootstrap")
        
        if not fed_a_addr or not fed_b_addr:
            print("Failed to get bootstrap addresses")
            return False
        
        # Create member nodes
        print("Creating member nodes...")
        fed_a_node1 = simulator.create_node("federation-a-node-1", "federation-a")
        fed_a_node2 = simulator.create_node("federation-a-node-2", "federation-a")
        fed_b_node1 = simulator.create_node("federation-b-node-1", "federation-b")
        fed_b_node2 = simulator.create_node("federation-b-node-2", "federation-b")
        
        # Start member nodes with different forwarding policies
        print("Starting federation A members...")
        simulator.start_node("federation-a-node-1", [fed_a_addr], "ForwardKnown")
        simulator.start_node("federation-a-node-2", [fed_a_addr], "ForwardKnown")
        
        print("Starting federation B members...")
        simulator.start_node("federation-b-node-1", [fed_b_addr], "ForwardKnown")
        simulator.start_node("federation-b-node-2", [fed_b_addr], "ForwardKnown")
        
        # Let the network stabilize
        print("Waiting for network to stabilize...")
        time.sleep(20)
        
        # Verify federation isolation
        print("Verifying federation isolation...")
        
        # Check that federations cannot communicate without a bridge
        for node_id in ["federation-a-node-1", "federation-a-node-2"]:
            logs = simulator.nodes[node_id].read_logs()
            if "federation-b" in logs:
                print(f"✗ Federation isolation failed: {node_id} has connections to federation B")
            else:
                print(f"✓ Federation isolation confirmed: {node_id} has no connections to federation B")
        
        # Now add a bridge with ForwardAll policy
        print("Creating and starting cross-federation bridge node...")
        bridge_node = simulator.create_node("cross-federation-bridge", "federation-a")
        simulator.start_node("cross-federation-bridge", [fed_a_addr, fed_b_addr], "ForwardAll")
        
        # Let the bridge establish connections
        print("Waiting for bridge to establish connections...")
        time.sleep(20)
        
        # Verify bridge connections
        bridge_logs = simulator.nodes["cross-federation-bridge"].read_logs()
        if (("Connected to peer" in bridge_logs and "federation-a" in bridge_logs) and 
            ("Connected to peer" in bridge_logs and "federation-b" in bridge_logs)):
            print("✓ Bridge node connected to both federations")
        else:
            print("✗ Bridge node failed to connect to both federations")
        
        print("Federation isolation test completed")
        return True
        
    except Exception as e:
        print(f"Error during federation isolation test: {e}")
        return False
    finally:
        # Stop all nodes
        print("Stopping all nodes...")
        simulator.stop_all_nodes()


def run_node_failure_test(simulator: TestnetSimulator):
    """Test resilience against node failures."""
    print("=== Running Node Failure Resilience Test ===")
    
    # Clean previous logs
    simulator.clean_logs()
    
    try:
        # Create federation bootstrap nodes
        print("Creating federation bootstrap nodes...")
        fed_a_bootstrap = simulator.create_node("federation-a-bootstrap", "federation-a")
        
        # Start federation bootstrap nodes
        print("Starting federation bootstrap node...")
        simulator.start_node("federation-a-bootstrap")
        
        # Wait for bootstrap nodes to initialize
        time.sleep(5)
        
        # Get bootstrap address
        fed_a_addr = simulator.get_node_address("federation-a-bootstrap")
        
        if not fed_a_addr:
            print("Failed to get bootstrap address")
            return False
        
        # Create member nodes
        print("Creating member nodes...")
        for i in range(1, 5):
            simulator.create_node(f"federation-a-node-{i}", "federation-a")
        
        # Start member nodes
        print("Starting federation members...")
        for i in range(1, 5):
            simulator.start_node(f"federation-a-node-{i}", [fed_a_addr])
        
        # Let the network stabilize
        print("Waiting for network to stabilize...")
        time.sleep(20)
        
        # Verify initial connectivity
        print("Verifying initial connectivity...")
        bootstrap_logs = simulator.nodes["federation-a-bootstrap"].read_logs()
        if "Connected to 4 peers" in bootstrap_logs or "Peers: 4" in bootstrap_logs:
            print("✓ Bootstrap node connected to all member nodes")
        else:
            print("✗ Bootstrap node not connected to all member nodes")
        
        # Simulate node failure
        print("Simulating node failure (stopping node-2)...")
        simulator.stop_node("federation-a-node-2")
        
        # Wait for network to adapt
        print("Waiting for network to adapt...")
        time.sleep(10)
        
        # Verify network adaptation
        print("Verifying network adaptation...")
        bootstrap_logs = simulator.nodes["federation-a-bootstrap"].read_logs()
        if "Peer disconnected" in bootstrap_logs:
            print("✓ Bootstrap node detected peer disconnection")
        else:
            print("? Bootstrap node might not have detected peer disconnection")
        
        # Start a new node
        print("Starting a new node to replace the failed one...")
        new_node = simulator.create_node("federation-a-replacement", "federation-a")
        simulator.start_node("federation-a-replacement", [fed_a_addr])
        
        # Wait for the new node to join
        print("Waiting for new node to join...")
        time.sleep(15)
        
        # Verify new node connection
        new_node_logs = simulator.nodes["federation-a-replacement"].read_logs()
        if "Connected to peer" in new_node_logs and fed_a_addr in new_node_logs:
            print("✓ New node successfully connected to the network")
        else:
            print("✗ New node failed to connect to the network")
        
        print("Node failure resilience test completed")
        return True
        
    except Exception as e:
        print(f"Error during node failure test: {e}")
        return False
    finally:
        # Stop all nodes
        print("Stopping all nodes...")
        simulator.stop_all_nodes()


def main():
    parser = argparse.ArgumentParser(description="ICN IPv6 Overlay Network Testnet Scenario Simulator")
    parser.add_argument("--scenario", choices=["basic", "isolation", "failure", "all"],
                      default="all", help="Test scenario to run")
    args = parser.parse_args()
    
    # Create simulator
    simulator = TestnetSimulator()
    
    # Ensure log directory exists
    os.makedirs(LOG_DIR, exist_ok=True)
    
    try:
        if args.scenario == "basic" or args.scenario == "all":
            run_basic_connectivity_test(simulator)
            if args.scenario != "all":
                return 0
        
        if args.scenario == "isolation" or args.scenario == "all":
            run_federation_isolation_test(simulator)
            if args.scenario != "all":
                return 0
        
        if args.scenario == "failure" or args.scenario == "all":
            run_node_failure_test(simulator)
            if args.scenario != "all":
                return 0
    
    finally:
        # Make sure all nodes are stopped
        simulator.stop_all_nodes()
    
    return 0


if __name__ == "__main__":
    exit(main()) 