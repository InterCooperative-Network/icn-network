#!/bin/bash
# Script to run the ICN Network demos

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}====================================================${NC}"
echo -e "${GREEN}ICN Network Demos${NC}"
echo -e "${BLUE}====================================================${NC}"

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo "cargo could not be found. Please install Rust."
    exit 1
fi

# Directory of this script
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
cd "$SCRIPT_DIR/.."

echo -e "${YELLOW}Building demos...${NC}"
cargo build --examples

echo -e "${BLUE}====================================================${NC}"
echo -e "${GREEN}Demo Options${NC}"
echo -e "${BLUE}====================================================${NC}"
echo "1) Run individual demos"
echo "2) Run integrated demo"
echo "3) Exit"
echo

read -p "Select an option: " option

case $option in
    1)
        echo -e "${BLUE}====================================================${NC}"
        echo -e "${GREEN}Individual Demos${NC}"
        echo -e "${BLUE}====================================================${NC}"
        echo "1) Metrics Demo"
        echo "2) Reputation Demo"
        echo "3) Priority Messaging Demo"
        echo "4) Back to main menu"
        echo

        read -p "Select a demo to run: " demo_option

        case $demo_option in
            1)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Metrics Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"

                echo "The metrics demo will start a web server at http://127.0.0.1:9090/metrics"
                echo "You can open this URL in your browser to see the metrics in real-time."
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo

                # Give user a moment to read instructions
                sleep 3

                # Run the metrics demo
                cargo run --example metrics_demo
                ;;
            2)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Reputation Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"

                echo "This demo will show how reputation affects peer connections"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo

                # Give user a moment to read instructions
                sleep 3

                # Run the reputation demo
                cargo run --example reputation_demo
                ;;
            3)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Priority Messaging Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"

                echo "This demo will show how message prioritization works based on reputation"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo

                # Give user a moment to read instructions
                sleep 3

                # Run the priority messaging demo
                cargo run --example priority_messaging
                ;;
            4)
                # Re-run this script
                exec $0
                ;;
            *)
                echo "Invalid option"
                exit 1
                ;;
        esac
        ;;
    2)
        echo -e "${BLUE}====================================================${NC}"
        echo -e "${GREEN}Integrated Demo Options${NC}"
        echo -e "${BLUE}====================================================${NC}"
        echo "1) Run complete integrated demo (metrics, reputation, priority)"
        echo "2) Run only metrics part"
        echo "3) Run only reputation part"
        echo "4) Run only priority part"
        echo "5) Back to main menu"
        echo
        
        read -p "Select an option: " integrated_option
        
        case $integrated_option in
            1)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Complete Integrated Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"
                
                echo "This demo will run all features: metrics, reputation, and priority messaging"
                echo "Metrics will be available at http://127.0.0.1:9090/metrics"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo
                
                # Give user a moment to read instructions
                sleep 3
                
                # Run the integrated demo with all features
                cargo run --example integrated_demo all
                ;;
            2)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Integrated Metrics Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"
                
                echo "This demo will focus on the metrics part of the integrated demo"
                echo "Metrics will be available at http://127.0.0.1:9090/metrics"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo
                
                # Give user a moment to read instructions
                sleep 3
                
                # Run the integrated demo with metrics
                cargo run --example integrated_demo metrics
                ;;
            3)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Integrated Reputation Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"
                
                echo "This demo will focus on the reputation part of the integrated demo"
                echo "Metrics will be available at http://127.0.0.1:9090/metrics"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo
                
                # Give user a moment to read instructions
                sleep 3
                
                # Run the integrated demo with reputation
                cargo run --example integrated_demo reputation
                ;;
            4)
                echo -e "${BLUE}====================================================${NC}"
                echo -e "${GREEN}Running Integrated Priority Demo${NC}"
                echo -e "${BLUE}====================================================${NC}"
                
                echo "This demo will focus on the priority messaging part of the integrated demo"
                echo "Metrics will be available at http://127.0.0.1:9090/metrics"
                echo -e "${YELLOW}Press Ctrl+C to stop the demo.${NC}"
                echo
                
                # Give user a moment to read instructions
                sleep 3
                
                # Run the integrated demo with priority
                cargo run --example integrated_demo priority
                ;;
            5)
                # Re-run this script
                exec $0
                ;;
            *)
                echo "Invalid option"
                exit 1
                ;;
        esac
        ;;
    3)
        echo "Exiting..."
        exit 0
        ;;
    *)
        echo "Invalid option"
        exit 1
        ;;
esac

echo
echo -e "${BLUE}====================================================${NC}"
echo -e "${GREEN}Demo completed!${NC}"
echo -e "${BLUE}====================================================${NC}" 