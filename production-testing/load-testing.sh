#!/run/current-system/sw/bin/bash

# DOTS Family Load Testing Script
# Performance and stress testing for production deployment

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

LOAD_TEST_REPORT="/tmp/dots-family-load-test-$(date +%Y%m%d-%H%M%S).txt"
TEST_DURATION=300  # 5 minutes
CONCURRENT_USERS=5
MAX_MEMORY_MB=512
MAX_CPU_PERCENT=80

log() {
    local level="$1"
    local message="$2"
    local timestamp=$(date +'%Y-%m-%d %H:%M:%S')
    
    case "$level" in
        "PASS")
            echo -e "${GREEN}[PASS]${NC} $message" | tee -a "$LOAD_TEST_REPORT"
            ;;
        "FAIL")
            echo -e "${RED}[FAIL]${NC} $message" | tee -a "$LOAD_TEST_REPORT"
            ;;
        "WARN")
            echo -e "${YELLOW}[WARN]${NC} $message" | tee -a "$LOAD_TEST_REPORT"
            ;;
        "INFO")
            echo -e "${BLUE}[INFO]${NC} $message" | tee -a "$LOAD_TEST_REPORT"
            ;;
    esac
}

check_prerequisites() {
    log "INFO" "Checking load testing prerequisites..."
    
    if ! command -v stress-ng >/dev/null 2>&1; then
        log "WARN" "stress-ng not available, using built-in stress testing"
    fi
    
    if ! command -v htop >/dev/null 2>&1 && ! command -v top >/dev/null 2>&1; then
        log "WARN" "No system monitoring tools available"
    fi
    
    # Check if daemon is running
    if ! pgrep -f dots-family-daemon >/dev/null; then
        log "FAIL" "DOTS Family daemon not running - cannot perform load tests"
        exit 1
    fi
    
    log "PASS" "Prerequisites check completed"
}

start_resource_monitoring() {
    log "INFO" "Starting resource monitoring..."
    
    # Start CPU and memory monitoring in background
    {
        while true; do
            local timestamp=$(date +'%Y-%m-%d %H:%M:%S')
            local pid=$(pgrep -f dots-family-daemon | head -1)
            
            if [[ -n "$pid" ]]; then
                # Get CPU and memory usage for daemon
                local cpu_mem=$(ps -p "$pid" -o %cpu,%mem,rss --no-headers 2>/dev/null || echo "0.0 0.0 0")
                local cpu=$(echo "$cpu_mem" | awk '{print $1}')
                local mem_percent=$(echo "$cpu_mem" | awk '{print $2}')
                local mem_kb=$(echo "$cpu_mem" | awk '{print $3}')
                local mem_mb=$((mem_kb / 1024))
                
                echo "$timestamp,daemon,$cpu,$mem_percent,$mem_mb" >> /tmp/dots-family-resources.csv
            fi
            
            # System-wide resource usage
            local load_avg=$(uptime | awk -F'load average:' '{print $2}' | cut -d',' -f1 | xargs)
            local mem_info=$(free | grep Mem)
            local mem_total=$(echo "$mem_info" | awk '{print $2}')
            local mem_used=$(echo "$mem_info" | awk '{print $3}')
            local mem_percent=$((mem_used * 100 / mem_total))
            
            echo "$timestamp,system,$load_avg,$mem_percent,0" >> /tmp/dots-family-resources.csv
            
            sleep 5
        done
    } &
    
    MONITOR_PID=$!
    echo "CPU,Memory,RSS" > /tmp/dots-family-resources.csv
    log "INFO" "Resource monitoring started (PID: $MONITOR_PID)"
}

stop_resource_monitoring() {
    if [[ -n "${MONITOR_PID:-}" ]]; then
        kill "$MONITOR_PID" 2>/dev/null || true
        wait "$MONITOR_PID" 2>/dev/null || true
        log "INFO" "Resource monitoring stopped"
    fi
}

simulate_user_activity() {
    local user_id="$1"
    local duration="$2"
    local start_time=$(date +%s)
    local end_time=$((start_time + duration))
    local activity_count=0
    
    log "INFO" "Starting user simulation $user_id for ${duration}s"
    
    while [[ $(date +%s) -lt $end_time ]]; do
        # Simulate various CLI operations
        local operations=(
            "status"
            "profile list"
            "check firefox"
            "check steam"
        )
        
        for op in "${operations[@]}"; do
            if command -v dots-family-ctl >/dev/null 2>&1; then
                dots-family-ctl $op >/dev/null 2>&1 || true
                ((activity_count++))
            fi
            
            # Add some delay between operations
            sleep $(( (RANDOM % 3) + 1 ))
            
            # Check if time is up
            if [[ $(date +%s) -ge $end_time ]]; then
                break
            fi
        done
    done
    
    log "INFO" "User simulation $user_id completed ($activity_count operations)"
}

run_concurrent_user_simulation() {
    log "INFO" "Starting concurrent user simulation ($CONCURRENT_USERS users, ${TEST_DURATION}s)"
    
    local pids=()
    
    # Start multiple user simulations
    for i in $(seq 1 "$CONCURRENT_USERS"); do
        simulate_user_activity "$i" "$TEST_DURATION" &
        pids+=($!)
    done
    
    # Wait for all simulations to complete
    for pid in "${pids[@]}"; do
        wait "$pid" 2>/dev/null || true
    done
    
    log "PASS" "Concurrent user simulation completed"
}

run_memory_stress_test() {
    log "INFO" "Running memory stress test..."
    
    local initial_memory=$(ps -p "$(pgrep -f dots-family-daemon)" -o rss --no-headers | head -1)
    initial_memory=$((initial_memory / 1024))  # Convert to MB
    
    # Create stress by rapidly querying the daemon
    {
        for i in $(seq 1 1000); do
            if command -v dots-family-ctl >/dev/null 2>&1; then
                dots-family-ctl status >/dev/null 2>&1 || true
                dots-family-ctl profile list >/dev/null 2>&1 || true
            fi
            
            # Small delay to avoid overwhelming
            sleep 0.1
        done
    } &
    
    local stress_pid=$!
    sleep 10  # Let stress run for 10 seconds
    kill "$stress_pid" 2>/dev/null || true
    wait "$stress_pid" 2>/dev/null || true
    
    # Check final memory usage
    sleep 2  # Allow memory to stabilize
    local final_memory=$(ps -p "$(pgrep -f dots-family-daemon)" -o rss --no-headers | head -1)
    final_memory=$((final_memory / 1024))  # Convert to MB
    
    local memory_increase=$((final_memory - initial_memory))
    
    log "INFO" "Memory usage: ${initial_memory}MB -> ${final_memory}MB (change: ${memory_increase}MB)"
    
    if [[ $final_memory -le $MAX_MEMORY_MB ]]; then
        log "PASS" "Memory usage within limits (${final_memory}MB <= ${MAX_MEMORY_MB}MB)"
    else
        log "FAIL" "Memory usage exceeded limits (${final_memory}MB > ${MAX_MEMORY_MB}MB)"
    fi
    
    if [[ $memory_increase -lt 50 ]]; then
        log "PASS" "Memory increase acceptable (${memory_increase}MB)"
    else
        log "WARN" "High memory increase during stress test (${memory_increase}MB)"
    fi
}

run_cpu_stress_test() {
    log "INFO" "Running CPU stress test..."
    
    # Start CPU-intensive operations
    {
        for i in $(seq 1 50); do
            if command -v dots-family-ctl >/dev/null 2>&1; then
                # Rapid-fire operations
                for j in $(seq 1 20); do
                    dots-family-ctl status >/dev/null 2>&1 || true
                done
            fi
        done
    } &
    
    local stress_pid=$!
    
    # Monitor CPU usage during stress
    local max_cpu=0
    for i in $(seq 1 10); do
        sleep 1
        local cpu_usage=$(ps -p "$(pgrep -f dots-family-daemon)" -o %cpu --no-headers | head -1 | xargs)
        cpu_usage=${cpu_usage%.*}  # Remove decimal part
        
        if [[ $cpu_usage -gt $max_cpu ]]; then
            max_cpu=$cpu_usage
        fi
    done
    
    kill "$stress_pid" 2>/dev/null || true
    wait "$stress_pid" 2>/dev/null || true
    
    log "INFO" "Maximum CPU usage during stress: ${max_cpu}%"
    
    if [[ $max_cpu -le $MAX_CPU_PERCENT ]]; then
        log "PASS" "CPU usage within limits (${max_cpu}% <= ${MAX_CPU_PERCENT}%)"
    else
        log "WARN" "CPU usage exceeded expected limits (${max_cpu}% > ${MAX_CPU_PERCENT}%)"
    fi
}

test_dbus_performance() {
    log "INFO" "Testing DBus interface performance..."
    
    local start_time=$(date +%s%3N)  # milliseconds
    local operation_count=100
    
    # Perform multiple DBus operations
    for i in $(seq 1 $operation_count); do
        if command -v dots-family-ctl >/dev/null 2>&1; then
            dots-family-ctl status >/dev/null 2>&1 || true
        fi
    done
    
    local end_time=$(date +%s%3N)
    local total_time=$((end_time - start_time))
    local avg_time=$((total_time / operation_count))
    
    log "INFO" "DBus performance: ${operation_count} operations in ${total_time}ms (avg: ${avg_time}ms)"
    
    if [[ $avg_time -lt 50 ]]; then
        log "PASS" "DBus response time excellent (<50ms average)"
    elif [[ $avg_time -lt 100 ]]; then
        log "PASS" "DBus response time good (<100ms average)"
    elif [[ $avg_time -lt 200 ]]; then
        log "WARN" "DBus response time acceptable (<200ms average)"
    else
        log "FAIL" "DBus response time poor (${avg_time}ms average)"
    fi
}

test_database_performance() {
    log "INFO" "Testing database performance..."
    
    # Simulate database-heavy operations
    local start_time=$(date +%s%3N)
    
    for i in $(seq 1 20); do
        if command -v dots-family-ctl >/dev/null 2>&1; then
            # Operations that likely hit the database
            dots-family-ctl profile list >/dev/null 2>&1 || true
            dots-family-ctl check firefox >/dev/null 2>&1 || true
        fi
    done
    
    local end_time=$(date +%s%3N)
    local total_time=$((end_time - start_time))
    
    log "INFO" "Database operations: 20 operations in ${total_time}ms"
    
    if [[ $total_time -lt 1000 ]]; then
        log "PASS" "Database performance excellent (<1s for 20 operations)"
    elif [[ $total_time -lt 2000 ]]; then
        log "PASS" "Database performance good (<2s for 20 operations)"
    else
        log "WARN" "Database performance may need optimization (${total_time}ms for 20 operations)"
    fi
}

generate_performance_report() {
    log "INFO" "Generating performance analysis..."
    
    # Analyze resource usage data
    if [[ -f "/tmp/dots-family-resources.csv" ]]; then
        local max_cpu=$(tail -n +2 /tmp/dots-family-resources.csv | grep "daemon" | cut -d',' -f3 | sort -nr | head -1 || echo "0")
        local max_memory=$(tail -n +2 /tmp/dots-family-resources.csv | grep "daemon" | cut -d',' -f5 | sort -nr | head -1 || echo "0")
        local avg_cpu=$(tail -n +2 /tmp/dots-family-resources.csv | grep "daemon" | cut -d',' -f3 | awk '{sum+=$1} END {print sum/NR}' || echo "0")
        
        echo "=== PERFORMANCE SUMMARY ===" | tee -a "$LOAD_TEST_REPORT"
        echo "Peak CPU Usage: ${max_cpu}%" | tee -a "$LOAD_TEST_REPORT"
        echo "Average CPU Usage: ${avg_cpu}%" | tee -a "$LOAD_TEST_REPORT"
        echo "Peak Memory Usage: ${max_memory}MB" | tee -a "$LOAD_TEST_REPORT"
        
        # Performance rating
        if (( $(echo "$max_cpu < 20" | bc -l) )) && [[ $max_memory -lt 100 ]]; then
            echo "Performance Rating: EXCELLENT" | tee -a "$LOAD_TEST_REPORT"
        elif (( $(echo "$max_cpu < 50" | bc -l) )) && [[ $max_memory -lt 200 ]]; then
            echo "Performance Rating: GOOD" | tee -a "$LOAD_TEST_REPORT"
        elif (( $(echo "$max_cpu < 80" | bc -l) )) && [[ $max_memory -lt 400 ]]; then
            echo "Performance Rating: ACCEPTABLE" | tee -a "$LOAD_TEST_REPORT"
        else
            echo "Performance Rating: NEEDS OPTIMIZATION" | tee -a "$LOAD_TEST_REPORT"
        fi
    fi
    
    # Clean up temporary files
    rm -f /tmp/dots-family-resources.csv
}

main() {
    echo "DOTS Family Mode Load Testing" | tee "$LOAD_TEST_REPORT"
    echo "=============================" | tee -a "$LOAD_TEST_REPORT"
    echo "Started: $(date)" | tee -a "$LOAD_TEST_REPORT"
    echo "Test Duration: ${TEST_DURATION}s" | tee -a "$LOAD_TEST_REPORT"
    echo "Concurrent Users: $CONCURRENT_USERS" | tee -a "$LOAD_TEST_REPORT"
    echo "" | tee -a "$LOAD_TEST_REPORT"
    
    # Set up signal handlers for cleanup
    trap 'stop_resource_monitoring; exit' INT TERM
    
    check_prerequisites
    start_resource_monitoring
    
    # Run performance tests
    run_concurrent_user_simulation
    run_memory_stress_test
    run_cpu_stress_test
    test_dbus_performance
    test_database_performance
    
    stop_resource_monitoring
    generate_performance_report
    
    echo "" | tee -a "$LOAD_TEST_REPORT"
    echo "Load testing completed: $(date)" | tee -a "$LOAD_TEST_REPORT"
    echo "Report saved to: $LOAD_TEST_REPORT" | tee -a "$LOAD_TEST_REPORT"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --duration)
            TEST_DURATION="$2"
            shift 2
            ;;
        --users)
            CONCURRENT_USERS="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --duration SECONDS  Test duration in seconds (default: 300)"
            echo "  --users NUMBER      Concurrent users to simulate (default: 5)"
            echo "  --help              Show this help message"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

main "$@"