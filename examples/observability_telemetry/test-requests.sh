#!/bin/bash

# Test script for ASH-RPC Observability Demo
# Makes 100+ curl requests to generate load and demonstrate metrics

set -e

SERVER_URL="http://localhost:3000/rpc"
BOLD='\033[1m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BOLD}${BLUE}=== ASH-RPC Observability Load Test ===${NC}"
echo -e "${BLUE}Making 100+ requests to generate metrics load...${NC}\n"

REQUEST_COUNT=0

# Function to make RPC request
make_request() {
    local method=$1
    local params=$2
    
    REQUEST_COUNT=$((REQUEST_COUNT + 1))
    
    if [ -z "$params" ]; then
        curl -s -X POST "$SERVER_URL" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"id\":$REQUEST_COUNT}" > /dev/null
    else
        curl -s -X POST "$SERVER_URL" \
            -H "Content-Type: application/json" \
            -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":$REQUEST_COUNT}" > /dev/null
    fi
    
    # Show progress every 10 requests
    if [ $((REQUEST_COUNT % 10)) -eq 0 ]; then
        echo -e "${YELLOW}Sent $REQUEST_COUNT requests...${NC}"
    fi
}

# Run 10 iterations of all request types
for iteration in {1..10}; do
    echo -e "${BLUE}Iteration $iteration/10${NC}"
    
    # Ping requests (3x)
    for i in {1..3}; do
        make_request "ping" ""
    done
    
    # Echo requests (3x)
    make_request "echo" "\"Hello World\""
    make_request "echo" "{\"test\":\"data\"}"
    make_request "echo" "[1,2,3,4,5]"
    
    # Math operations (4x)
    make_request "add" "[10,5]"
    make_request "add" "[$RANDOM,$RANDOM]"
    make_request "multiply" "[7,8]"
    make_request "multiply" "[$((RANDOM % 100)),$((RANDOM % 100))]"
    
    # Slow operations (1x per iteration)
    make_request "slow_operation" ""
    
    # Error cases (2x)
    make_request "always_fails" ""
    make_request "add" "[1]"  # Invalid params
done

echo -e "\n${BOLD}${GREEN}Load test completed!${NC}"
echo -e "${BLUE}Total requests sent: ${BOLD}$REQUEST_COUNT${NC}"
echo -e "${BLUE}Check Grafana dashboard at: ${BOLD}http://localhost:3001${NC}"
echo -e "${BLUE}  Username: admin${NC}"
echo -e "${BLUE}  Password: admin${NC}"
echo -e "${BLUE}Check Jaeger traces at: ${BOLD}http://localhost:16686${NC}"
echo -e "${BLUE}Check Prometheus at: ${BOLD}http://localhost:9090${NC}"
echo -e "${BLUE}Check metrics endpoint at: ${BOLD}http://localhost:3000/metrics${NC}\n"
