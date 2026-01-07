#!/bin/bash

# 📊 System Monitoring Script for Rust Edge Compute Framework
# Monitors CPU, memory, disk, and service health
# Usage: ./deploy/monitoring/monitor.sh [OPTIONS]

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
SERVICE_NAME="rust-edge-compute"
LOG_FILE="/var/log/edge-compute/monitor.log"
ALERT_THRESHOLD_CPU=80
ALERT_THRESHOLD_MEM=80
ALERT_THRESHOLD_DISK=90
CHECK_INTERVAL=60
ENABLE_ALERTS=true

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
    --interval <SECONDS>    Check interval (default: 60)
    --cpu-threshold <PCT>   CPU alert threshold (default: 80%)
    --mem-threshold <PCT>   Memory alert threshold (default: 80%)
    --disk-threshold <PCT>  Disk alert threshold (default: 90%)
    --no-alerts            Disable alert notifications
    --log-file <PATH>      Log file path (default: /var/log/edge-compute/monitor.log)
    --help                 Show this help message

Examples:
    ./deploy/monitoring/monitor.sh
    ./deploy/monitoring/monitor.sh --interval 30 --cpu-threshold 70
    ./deploy/monitoring/monitor.sh --no-alerts --log-file /tmp/monitor.log
EOF
    exit 0
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --interval)
            CHECK_INTERVAL="$2"
            shift 2
            ;;
        --cpu-threshold)
            ALERT_THRESHOLD_CPU="$2"
            shift 2
            ;;
        --mem-threshold)
            ALERT_THRESHOLD_MEM="$2"
            shift 2
            ;;
        --disk-threshold)
            ALERT_THRESHOLD_DISK="$2"
            shift 2
            ;;
        --no-alerts)
            ENABLE_ALERTS=false
            shift
            ;;
        --log-file)
            LOG_FILE="$2"
            shift 2
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

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")"
touch "$LOG_FILE"

# Log message
log_message() {
    local level=$1
    local message=$2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [$level] $message" >> "$LOG_FILE"
}

# Send alert (placeholder - can be extended with email, Slack, etc.)
send_alert() {
    local severity=$1
    local message=$2
    
    if [ "$ENABLE_ALERTS" = false ]; then
        return
    fi
    
    log_message "ALERT" "[$severity] $message"
    
    # Placeholder for external notification
    # Uncomment to enable email or webhook alerts
    # curl -X POST http://alert-server/api/alerts \
    #   -d "severity=$severity&message=$message"
}

# Monitor function
monitor() {
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}📊 System Monitoring - $(date '+%Y-%m-%d %H:%M:%S')${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    
    # Check CPU usage
    echo -e "\n${YELLOW}CPU Usage:${NC}"
    if command -v top &> /dev/null; then
        CPU_LOAD=$(uptime | awk '{print $(NF-2)}' | sed 's/,//')
        echo "  Load Average: $CPU_LOAD"
        
        # Get per-core usage
        if [ -f /proc/stat ]; then
            # Simple CPU percentage estimation
            CPU_PERCENT=$(top -bn1 | grep "Cpu(s)" | awk '{print int($2)}' || echo "0")
            echo "  Overall Usage: ${CPU_PERCENT}%"
            
            if [ "$CPU_PERCENT" -gt "$ALERT_THRESHOLD_CPU" ]; then
                echo -e "${RED}  ⚠️  HIGH CPU USAGE!${NC}"
                send_alert "WARNING" "CPU usage at ${CPU_PERCENT}% (threshold: ${ALERT_THRESHOLD_CPU}%)"
            else
                echo -e "${GREEN}  ✓ OK${NC}"
            fi
        fi
    fi
    
    # Check Memory usage
    echo -e "\n${YELLOW}Memory Usage:${NC}"
    if command -v free &> /dev/null; then
        MEMORY_INFO=$(free | awk 'NR==2{printf("%.1f", $3/$2 * 100)}')
        MEMORY_USED=$(free -h | awk 'NR==2{print $3}')
        MEMORY_TOTAL=$(free -h | awk 'NR==2{print $2}')
        
        echo "  Usage: $MEMORY_USED / $MEMORY_TOTAL (${MEMORY_INFO}%)"
        
        if (( $(echo "$MEMORY_INFO > $ALERT_THRESHOLD_MEM" | bc -l) )); then
            echo -e "${RED}  ⚠️  HIGH MEMORY USAGE!${NC}"
            send_alert "WARNING" "Memory usage at ${MEMORY_INFO}% (threshold: ${ALERT_THRESHOLD_MEM}%)"
        else
            echo -e "${GREEN}  ✓ OK${NC}"
        fi
    fi
    
    # Check Disk usage
    echo -e "\n${YELLOW}Disk Usage:${NC}"
    if command -v df &> /dev/null; then
        df -h /opt/edge-compute 2>/dev/null | awk 'NR==2{
            printf("  Mount: %s\n", $6);
            printf("  Usage: %s / %s (%s)\n", $3, $2, $5);
            gsub(/%/, "", $5);
            if ($5 > '$ALERT_THRESHOLD_DISK') {
                print "  ⚠️  HIGH DISK USAGE!";
                system("echo \"ALERT\" > /tmp/disk_alert");
            } else {
                print "  ✓ OK";
            }
        }' || echo "  Cannot read disk usage"
        
        if [ -f /tmp/disk_alert ]; then
            rm /tmp/disk_alert
            send_alert "WARNING" "Disk usage above ${ALERT_THRESHOLD_DISK}% threshold"
        fi
    fi
    
    # Check service status
    echo -e "\n${YELLOW}Service Status:${NC}"
    if systemctl is-active --quiet $SERVICE_NAME; then
        echo -e "  Status: ${GREEN}Active${NC}"
        
        # Get service metrics
        if systemctl show $SERVICE_NAME --property=ActiveEnterTimestamp --value > /dev/null 2>&1; then
            UPTIME=$(systemctl show $SERVICE_NAME --property=ActiveEnterTimestamp --value)
            echo "  Active Since: $UPTIME"
        fi
        
        # Check recent errors
        ERROR_COUNT=$(journalctl -u $SERVICE_NAME -n 100 2>/dev/null | grep -ic "error" || echo "0")
        if [ "$ERROR_COUNT" -gt 0 ]; then
            echo -e "  Errors in logs: ${RED}$ERROR_COUNT${NC}"
            send_alert "INFO" "Found $ERROR_COUNT errors in service logs"
        fi
    else
        echo -e "  Status: ${RED}Inactive${NC}"
        send_alert "CRITICAL" "Service $SERVICE_NAME is not running"
    fi
    
    # Check process details
    echo -e "\n${YELLOW}Process Details:${NC}"
    if pgrep -f rust-edge-compute > /dev/null; then
        PID=$(pgrep -f rust-edge-compute | head -1)
        echo "  PID: $PID"
        
        if [ -f /proc/$PID/status ]; then
            PROCESS_MEM=$(awk '/VmRSS/{print $2}' /proc/$PID/status)
            PROCESS_CPU=$(ps -p $PID -o %cpu --no-headers)
            echo "  Memory: ${PROCESS_MEM}KB"
            echo "  CPU: ${PROCESS_CPU}%"
        fi
    else
        echo -e "  ${RED}Process not running${NC}"
    fi
    
    # Summary
    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}📋 Summary:${NC}"
    echo "  Check Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "  Log File: $LOG_FILE"
    echo "  Next Check: In $CHECK_INTERVAL seconds"
    
    log_message "INFO" "Monitoring check completed"
}

# Continuous monitoring
echo -e "${YELLOW}🔄 Starting continuous monitoring (Press Ctrl+C to stop)${NC}"
echo -e "${YELLOW}Check Interval: ${CHECK_INTERVAL}s${NC}"
echo -e "${YELLOW}Thresholds - CPU: ${ALERT_THRESHOLD_CPU}%, Memory: ${ALERT_THRESHOLD_MEM}%, Disk: ${ALERT_THRESHOLD_DISK}%${NC}\n"

log_message "INFO" "Monitoring started with thresholds: CPU=$ALERT_THRESHOLD_CPU%, MEM=$ALERT_THRESHOLD_MEM%, DISK=$ALERT_THRESHOLD_DISK%"

while true; do
    monitor
    sleep "$CHECK_INTERVAL"
done
