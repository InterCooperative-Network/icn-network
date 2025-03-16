#!/usr/bin/env python3
"""
ICN IPv6 Overlay Network Testnet Monitor

This script monitors the testnet logs and visualizes the network status
and connections between nodes.
"""

import os
import re
import time
import json
import argparse
import subprocess
from datetime import datetime
from collections import defaultdict
import curses
from curses import wrapper

# Configuration
LOG_DIR = "./logs"
REFRESH_INTERVAL = 2  # seconds

# Regular expressions for log parsing
NODE_INITIALIZED_RE = re.compile(r"Node initialized with overlay address: (.*)")
PEER_CONNECTED_RE = re.compile(r"Connected to peer: (.*)")
TUNNEL_CREATED_RE = re.compile(r"Created tunnel to (.*) using (.*)")
MESSAGE_SENT_RE = re.compile(r"Sending .* to (.*): (.*)")
MESSAGE_RECEIVED_RE = re.compile(r"Received message from (.*): (.*)")

# Data structures
nodes = {}  # node_id -> node_info
connections = {}  # (source, dest) -> connection_info
messages = []  # list of message_info


class Node:
    def __init__(self, node_id, federation=None):
        self.node_id = node_id
        self.federation = federation
        self.address = None
        self.peers = set()
        self.tunnels = set()
        self.messages_sent = 0
        self.messages_received = 0
        self.last_seen = 0
        self.status = "unknown"

    def update_status(self):
        now = time.time()
        if now - self.last_seen > 30:
            self.status = "offline"
        elif now - self.last_seen > 10:
            self.status = "inactive"
        else:
            self.status = "active"


class Connection:
    def __init__(self, source, dest):
        self.source = source
        self.dest = dest
        self.tunnel_type = None
        self.messages = 0
        self.last_message = 0


class Message:
    def __init__(self, source, dest, content, timestamp):
        self.source = source
        self.dest = dest
        self.content = content
        self.timestamp = timestamp


def parse_node_id_from_filename(filename):
    """Extract node ID from log filename."""
    base = os.path.basename(filename)
    return os.path.splitext(base)[0]


def parse_federation_from_node_id(node_id):
    """Extract federation from node ID."""
    if "federation-a" in node_id:
        return "federation-a"
    elif "federation-b" in node_id:
        return "federation-b"
    elif "federation-c" in node_id:
        return "federation-c"
    elif "cross-federation" in node_id:
        parts = node_id.split("-")
        if len(parts) >= 3:
            if parts[-1] == "ab":
                return "federation-a+federation-b"
            elif parts[-1] == "bc":
                return "federation-b+federation-c"
            elif parts[-1] == "ac":
                return "federation-a+federation-c"
    return None


def parse_logs():
    """Parse all log files and update data structures."""
    for filename in os.listdir(LOG_DIR):
        if not filename.endswith(".log"):
            continue

        filepath = os.path.join(LOG_DIR, filename)
        node_id = parse_node_id_from_filename(filename)
        federation = parse_federation_from_node_id(node_id)

        # Create node if it doesn't exist
        if node_id not in nodes:
            nodes[node_id] = Node(node_id, federation)

        # Update node's last seen time
        if os.path.exists(filepath):
            nodes[node_id].last_seen = os.path.getmtime(filepath)

        # Parse log file
        with open(filepath, "r") as f:
            for line in f:
                # Extract timestamp
                timestamp_match = re.match(r"\[(.*?)\]", line)
                timestamp = None
                if timestamp_match:
                    try:
                        timestamp_str = timestamp_match.group(1).split()[0]
                        timestamp = datetime.strptime(timestamp_str, "%Y-%m-%d")
                    except (ValueError, IndexError):
                        pass

                # Check for node initialization
                match = NODE_INITIALIZED_RE.search(line)
                if match:
                    nodes[node_id].address = match.group(1)
                    continue

                # Check for peer connections
                match = PEER_CONNECTED_RE.search(line)
                if match:
                    peer_addr = match.group(1)
                    nodes[node_id].peers.add(peer_addr)
                    
                    # Add connection
                    for other_id, other_node in nodes.items():
                        if other_node.address == peer_addr:
                            conn_key = (node_id, other_id)
                            if conn_key not in connections:
                                connections[conn_key] = Connection(node_id, other_id)
                    continue

                # Check for tunnel creation
                match = TUNNEL_CREATED_RE.search(line)
                if match:
                    tunnel_dest = match.group(1)
                    tunnel_type = match.group(2)
                    nodes[node_id].tunnels.add(tunnel_dest)
                    
                    # Update connection with tunnel info
                    for other_id, other_node in nodes.items():
                        if other_node.address == tunnel_dest:
                            conn_key = (node_id, other_id)
                            if conn_key in connections:
                                connections[conn_key].tunnel_type = tunnel_type
                            else:
                                conn = Connection(node_id, other_id)
                                conn.tunnel_type = tunnel_type
                                connections[conn_key] = conn
                    continue

                # Check for message sent
                match = MESSAGE_SENT_RE.search(line)
                if match:
                    dest_addr = match.group(1)
                    message_content = match.group(2)
                    nodes[node_id].messages_sent += 1
                    
                    # Update connection and add message
                    for other_id, other_node in nodes.items():
                        if other_node.address == dest_addr:
                            conn_key = (node_id, other_id)
                            if conn_key in connections:
                                connections[conn_key].messages += 1
                                connections[conn_key].last_message = time.time()
                            
                            # Add to recent messages
                            msg = Message(node_id, other_id, message_content, timestamp or time.time())
                            messages.append(msg)
                            if len(messages) > 100:  # Keep only last 100 messages
                                messages.pop(0)
                    continue

                # Check for message received
                match = MESSAGE_RECEIVED_RE.search(line)
                if match:
                    source_addr = match.group(1)
                    message_content = match.group(2)
                    nodes[node_id].messages_received += 1
                    continue

    # Update node statuses
    for node in nodes.values():
        node.update_status()


def draw_network_status(stdscr):
    """Draw network status in the terminal."""
    curses.start_color()
    curses.use_default_colors()
    curses.init_pair(1, curses.COLOR_GREEN, -1)  # Active
    curses.init_pair(2, curses.COLOR_YELLOW, -1)  # Inactive
    curses.init_pair(3, curses.COLOR_RED, -1)  # Offline
    curses.init_pair(4, curses.COLOR_CYAN, -1)  # Federation A
    curses.init_pair(5, curses.COLOR_MAGENTA, -1)  # Federation B
    curses.init_pair(6, curses.COLOR_BLUE, -1)  # Federation C
    curses.init_pair(7, curses.COLOR_WHITE, -1)  # No federation
    curses.init_pair(8, curses.COLOR_GREEN, curses.COLOR_BLACK)  # Headers

    # Clear screen
    stdscr.clear()
    height, width = stdscr.getmaxyx()

    # Draw header
    header = "ICN IPv6 Overlay Network Testnet Monitor"
    stdscr.addstr(0, (width - len(header)) // 2, header, curses.A_BOLD | curses.color_pair(8))
    stdscr.addstr(1, 0, "=" * width, curses.A_BOLD)

    # Draw node information
    stdscr.addstr(2, 2, "Nodes:", curses.A_BOLD)
    y = 3
    for node_id, node in sorted(nodes.items()):
        if y >= height - 5:
            break

        # Choose color based on status and federation
        status_color = curses.color_pair(1)  # Default: active
        if node.status == "inactive":
            status_color = curses.color_pair(2)
        elif node.status == "offline":
            status_color = curses.color_pair(3)

        fed_color = curses.color_pair(7)  # Default: no federation
        if node.federation == "federation-a" or "federation-a+" in node.federation:
            fed_color = curses.color_pair(4)
        elif node.federation == "federation-b" or "federation-b+" in node.federation:
            fed_color = curses.color_pair(5)
        elif node.federation == "federation-c" or "federation-c+" in node.federation:
            fed_color = curses.color_pair(6)

        # Draw node info
        stdscr.addstr(y, 2, f"{node_id}: ", fed_color)
        stdscr.addstr(f"[{node.status}] ", status_color)
        stdscr.addstr(f"Addr: {node.address or 'Unknown'} | ")
        stdscr.addstr(f"Peers: {len(node.peers)} | ")
        stdscr.addstr(f"Tunnels: {len(node.tunnels)} | ")
        stdscr.addstr(f"Msgs: Sent={node.messages_sent}, Recv={node.messages_received}")
        y += 1

    # Draw connection information
    y += 1
    if y < height - 5:
        stdscr.addstr(y, 2, "Active Connections:", curses.A_BOLD)
        y += 1
        active_connections = {k: v for k, v in connections.items() if v.last_message > time.time() - 60}
        for (src, dst), conn in sorted(active_connections.items()):
            if y >= height - 5:
                break
            stdscr.addstr(y, 2, f"{src} -> {dst} | ")
            stdscr.addstr(f"Tunnel: {conn.tunnel_type or 'None'} | ")
            stdscr.addstr(f"Messages: {conn.messages}")
            y += 1

    # Draw recent messages
    y += 1
    if y < height - 5:
        stdscr.addstr(y, 2, "Recent Messages:", curses.A_BOLD)
        y += 1
        recent_msgs = sorted(messages, key=lambda m: m.timestamp, reverse=True)[:10]
        for msg in recent_msgs:
            if y >= height - 3:
                break
            try:
                ts = datetime.fromtimestamp(msg.timestamp).strftime("%H:%M:%S")
            except:
                ts = "Unknown"
            content = msg.content
            if len(content) > width - 30:
                content = content[:width - 33] + "..."
            stdscr.addstr(y, 2, f"[{ts}] {msg.source} -> {msg.dest}: {content}")
            y += 1

    # Draw footer
    stdscr.addstr(height - 2, 0, "=" * width)
    footer = f"Last updated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')} | Press 'q' to quit"
    stdscr.addstr(height - 1, 2, footer)

    # Refresh the screen
    stdscr.refresh()


def monitor_loop(stdscr):
    """Main monitoring loop."""
    # Hide cursor
    curses.curs_set(0)
    
    # Enable keypad mode
    stdscr.keypad(True)
    
    # Set getch() to non-blocking
    stdscr.nodelay(True)
    
    while True:
        parse_logs()
        draw_network_status(stdscr)
        
        # Check for quit key
        c = stdscr.getch()
        if c == ord('q'):
            break
        
        time.sleep(REFRESH_INTERVAL)


def main():
    parser = argparse.ArgumentParser(description="ICN IPv6 Overlay Network Testnet Monitor")
    parser.add_argument("--log-dir", default=LOG_DIR, help="Directory containing log files")
    args = parser.parse_args()
    
    global LOG_DIR
    LOG_DIR = args.log_dir
    
    # Ensure log directory exists
    if not os.path.exists(LOG_DIR):
        print(f"Error: Log directory {LOG_DIR} does not exist")
        return 1
    
    # Run the monitor in curses wrapper
    wrapper(monitor_loop)
    
    return 0


if __name__ == "__main__":
    exit(main()) 