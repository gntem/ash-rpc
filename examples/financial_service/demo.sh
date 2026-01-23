#!/bin/bash

# Financial Service Demo Script
# Starts the service and demonstrates financial data access with audit logging

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Financial Service Demo with Audit Logging${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""

# Add db if not exists
if [ ! -f "sample.db" ]; then
    echo -e "${GREEN}Creating SQLite database and seeding...${NC}"
    sqlite3 sample.db <<'SQL'
        CREATE TABLE IF NOT EXISTS account(
        userid TEXT PRIMARY KEY,
        important_info TEXT
        );
SQL
fi

# Build the project
echo -e "${GREEN}Building project...${NC}"
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" || true
echo ""

# Kill any existing process on port 9001
echo -e "${YELLOW}Checking for existing processes on port 9001...${NC}"
EXISTING_PID=$(lsof -ti:9001 2>/dev/null || true)
if [ ! -z "$EXISTING_PID" ]; then
    echo -e "${YELLOW}Found existing process (PID: $EXISTING_PID), killing it...${NC}"
    kill -TERM $EXISTING_PID 2>/dev/null || true
    sleep 1
    # Force kill if still alive
    if kill -0 $EXISTING_PID 2>/dev/null; then
        kill -9 $EXISTING_PID 2>/dev/null || true
        sleep 0.5
    fi
fi

# Start the service in background
echo -e "${GREEN}Starting financial service on port 9001...${NC}"
nohup cargo run --release > /dev/null 2>&1 &
SERVICE_PID=$!

# Wait for service to be ready (check TCP port and healthcheck)
echo -e "${YELLOW}Waiting for service to start (checking localhost:9001)...${NC}"
ready=false
for i in {1..40}; do
    if nc -z localhost 9001 2>/dev/null; then
        # Port is open, now check healthcheck endpoint
        health_response=$(echo '{"jsonrpc":"2.0","method":"health","id":0}' | timeout 2 nc localhost 9001 2>/dev/null || echo "")
        if echo "$health_response" | grep -q '"status":"healthy"'; then
            ready=true
            break
        fi
    fi
    sleep 0.25
done

if [ "$ready" != true ]; then
    echo -e "${RED}Failed to start service (health check failed)${NC}"
    kill -TERM $SERVICE_PID 2>/dev/null || true
    sleep 1
    kill -9 $SERVICE_PID 2>/dev/null || true
    wait $SERVICE_PID 2>/dev/null || true
    exit 1
fi

echo -e "${GREEN}Service is ready! (pid=${SERVICE_PID})${NC}"
echo ""

# Function to make RPC call and show result
make_call() {
    local user=$1
    local id=$2
    
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}Accessing financial data for: ${YELLOW}$user${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    local request='{"jsonrpc":"2.0","method":"get_financial_information","params":{"accessor":"'$user'"},"id":'$id'}'
    echo -e "${YELLOW}Request:${NC}"
    echo "$request" | jq .
    
    echo -e "${YELLOW}Response:${NC}"
    echo "$request" | timeout 3 nc localhost 9001 | jq .
    
    sleep 1
    echo ""
}

# Test with all seeded accounts
make_call "alice" 1
make_call "bob" 2
make_call "charlie" 3
make_call "dave" 4

# Test with non-existent account
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}Attempting to access non-existent account${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
request='{"jsonrpc":"2.0","method":"get_financial_information","params":{"accessor":"eve"},"id":5}'
echo -e "${YELLOW}Request:${NC}"
echo "$request" | jq .
echo -e "${YELLOW}Response:${NC}"
echo "$request" | timeout 3 nc localhost 9001 | jq .
echo ""

# Show summary
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}Demo Complete!${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${YELLOW}All financial data access has been logged with:${NC}"
echo "  • Critical severity for successful access"
echo "  • Warning severity for failed attempts"
echo "  • Sequence numbers for integrity verification"
echo ""

# Cleanup
echo -e "${GREEN}Stopping service gracefully...${NC}"
kill -TERM $SERVICE_PID 2>/dev/null || true
sleep 2
# Force kill if still alive
if kill -0 $SERVICE_PID 2>/dev/null; then
    echo -e "${YELLOW}Service didn't stop gracefully, forcing...${NC}"
    kill -9 $SERVICE_PID 2>/dev/null || true
fi
wait $SERVICE_PID 2>/dev/null || true

echo -e "${GREEN}Done!${NC}"
