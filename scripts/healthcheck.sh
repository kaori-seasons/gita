#!/bin/bash

# 🏥 Health Check Script for Rust Edge Compute Framework
# Performs comprehensive health checks
# Usage: ./scripts/healthcheck.sh [OPTIONS]

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
API_URL="http://localhost:3000"
TIMEOUT=5
CHECK_INTERVAL=10
VERBOSE=false
WATCH_MODE=false

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
    --api-url <URL>       API endpoint (default: http://localhost:3000)
    --timeout <SECONDS>   Request timeout (default: 5)
    --interval <SECONDS>  Check interval for continuous monitoring (default: 10)
    --verbose             Show detailed output
    --watch               Continuous monitoring mode
    --help                Show this help message

Examples:
    ./scripts/healthcheck.sh                      # One-time health check
    ./scripts/healthcheck.sh --watch              # Continuous monitoring
    ./scripts/healthcheck.sh --api-url http://edge-node:3000 --watch
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --api-url)
            API_URL="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --interval)
            CHECK_INTERVAL="$2"
            shift 2
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --watch)
            WATCH_MODE=true
            shift
            ;;
        --help)
            usage
            ;;
        *)
            echo "Unknown option: $1"
            usage
            ;;
    esac
done

# Color codes for status
STATUS_OK="${GREEN}✅${NC}"
STATUS_FAIL="${RED}❌${NC}"
STATUS_WARN="${YELLOW}⚠️${NC}"

perform_health_check() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}🏥 Health Check - $(date '+%Y-%m-%d %H:%M:%S')${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    local all_passed=true
    
    # Check 1: Service connectivity
    echo -e "\n${YELLOW}1. API Connectivity${NC}"
    if curl -sf --max-time $TIMEOUT "$API_URL/api/v1/health" > /dev/null 2>&1; then
        echo -e "$STATUS_OK API is responding"
    else
        echo -e "$STATUS_FAIL Cannot reach API at $API_URL"
        all_passed=false
        return 1
    fi
    
    # Check 2: Health endpoint details
    echo -e "\n${YELLOW}2. Health Status Details${NC}"
    HEALTH_RESPONSE=$(curl -s --max-time $TIMEOUT "$API_URL/api/v1/health" 2>/dev/null || echo "{}")
    
    if [ "$VERBOSE" = true ]; then
        echo "$HEALTH_RESPONSE" | jq . 2>/dev/null || echo "$HEALTH_RESPONSE"
    fi
    
    # Extract status from response
    STATUS=$(echo "$HEALTH_RESPONSE" | jq -r '.status // "unknown"' 2>/dev/null)
    if [ "$STATUS" = "ok" ] || [ "$STATUS" = "OK" ]; then
        echo -e "$STATUS_OK Service status: $STATUS"
    else
        echo -e "$STATUS_WARN Service status: $STATUS"
    fi
    
    # Check 3: System resources
    echo -e "\n${YELLOW}3. System Resources${NC}"
    
    # Memory usage
    if command -v free &> /dev/null; then
        MEM_USAGE=$(free | awk 'NR==2{printf("%.1f", $3/$2 * 100)}')
        if (( $(echo "$MEM_USAGE < 80" | bc -l) )); then
            echo -e "$STATUS_OK Memory usage: ${MEM_USAGE}%"
        else
            echo -e "$STATUS_WARN Memory usage: ${MEM_USAGE}%"
        fi
    fi
    
    # Disk usage
    if command -v df &> /dev/null; then
        DISK_USAGE=$(df /opt/edge-compute 2>/dev/null | awk 'NR==2{printf("%d", $5)}' || echo "0")
        if [ "$DISK_USAGE" != "0" ]; then
            if [ "$DISK_USAGE" -lt 80 ]; then
                echo -e "$STATUS_OK Disk usage: ${DISK_USAGE}%"
            else
                echo -e "$STATUS_WARN Disk usage: ${DISK_USAGE}%"
            fi
        fi
    fi
    
    # CPU usage
    if command -v uptime &> /dev/null; then
        LOAD=$(uptime | awk '{print $(NF-2)}' | sed 's/,//')
        echo -e "${BLUE}📊${NC} CPU Load Average: $LOAD"
    fi
    
    # Check 4: Service status
    echo -e "\n${YELLOW}4. Service Status${NC}"
    if systemctl is-active --quiet rust-edge-compute 2>/dev/null; then
        echo -e "$STATUS_OK systemd service is active"
        
        # Check restart count
        RESTART_COUNT=$(systemctl show rust-edge-compute --property NRestarts --value 2>/dev/null || echo "0")
        echo -e "${BLUE}📊${NC} Service restarts: $RESTART_COUNT"
    else
        echo -e "$STATUS_WARN systemd service is not active"
        all_passed=false
    fi
    
    # Check 5: Recent logs
    echo -e "\n${YELLOW}5. Recent Logs${NC}"
    if command -v journalctl &> /dev/null; then
        ERROR_COUNT=$(journalctl -u rust-edge-compute -n 100 2>/dev/null | grep -ic "error" || echo "0")
        if [ "$ERROR_COUNT" -eq 0 ]; then
            echo -e "$STATUS_OK No errors in recent logs"
        else
            echo -e "$STATUS_WARN Found $ERROR_COUNT error(s) in logs"
            if [ "$VERBOSE" = true ]; then
                echo -e "\n${YELLOW}Recent errors:${NC}"
                journalctl -u rust-edge-compute -n 20 2>/dev/null | grep -i "error" | tail -5
            fi
        fi
    fi
    
    # Check 6: Database connectivity (if applicable)
    echo -e "\n${YELLOW}6. Data Integrity${NC}"
    DB_PATH="/opt/edge-compute/data/db"
    if [ -d "$DB_PATH" ]; then
        DB_SIZE=$(du -sh "$DB_PATH" 2>/dev/null | awk '{print $1}')
        echo -e "$STATUS_OK Database exists (size: $DB_SIZE)"
    fi
    
    # Summary
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    if [ "$all_passed" = true ]; then
        echo -e "${GREEN}✅ All health checks passed!${NC}"
        return 0
    else
        echo -e "${RED}❌ Some health checks failed!${NC}"
        return 1
    fi
}

# Main execution
if [ "$WATCH_MODE" = true ]; then
    echo -e "${YELLOW}🔄 Continuous monitoring mode (Press Ctrl+C to stop)${NC}\n"
    while true; do
        perform_health_check
        echo -e "\n${YELLOW}⏳ Next check in $CHECK_INTERVAL seconds...\n${NC}"
        sleep $CHECK_INTERVAL
    done
else
    perform_health_check
    exit $?
fi
