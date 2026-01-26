# ゲスト-ハイパーバイザー跨ぎ性能測定方法論

## 🎯 跨ぎ性能測定の課題と解決策

### 測定の複雑性

#### **複数レイヤーでの協調測定**
```
Application Layer    │ App Response Time
─────────────────────┼─────────────────
Guest OS Layer       │ System Call Latency
─────────────────────┼─────────────────
Hypervisor Layer     │ VMExit/VMEntry Time
─────────────────────┼─────────────────
Hardware Layer       │ Physical Resource Access
```

#### **測定の同期問題**
- **時刻同期**: 各レイヤーでの時刻基準の統一
- **因果関係**: イベントの因果関係の追跡
- **オーバーヘッド**: 測定自体による性能影響

## 📊 統合測定アーキテクチャ

### 測定インフラストラクチャ

#### **MiniVisorでの実装設計**
```rust
// src/performance_monitor.rs
use core::sync::atomic::{AtomicU64, Ordering};

pub struct CrossLayerProfiler {
    // ハイパーバイザー測定
    hypervisor_metrics: HypervisorMetrics,
    // ゲスト測定インターフェース
    guest_metrics: GuestMetricsInterface,
    // 統合分析エンジン
    correlation_engine: CorrelationEngine,
    // リアルタイム表示
    realtime_display: RealtimeDisplay,
}

#[derive(Debug)]
pub struct HypervisorMetrics {
    vmexit_count: AtomicU64,
    vmexit_total_cycles: AtomicU64,
    vmentry_total_cycles: AtomicU64,
    mmio_access_count: AtomicU64,
    interrupt_injection_count: AtomicU64,
    page_fault_count: AtomicU64,
    hypercall_count: AtomicU64,
}

#[derive(Debug)]
pub struct CrossLayerEvent {
    event_id: u64,
    timestamp: u64,
    layer: SystemLayer,
    event_type: EventType,
    guest_id: Option<u32>,
    duration: Option<u64>,
    related_events: Vec<u64>,
}

#[derive(Debug)]
pub enum SystemLayer {
    Application,
    GuestOS,
    Hypervisor,
    Hardware,
}

impl CrossLayerProfiler {
    pub fn record_vmexit(&self, guest_id: u32, reason: VMExitReason) -> EventId {
        let event_id = self.generate_event_id();
        let timestamp = self.read_cycle_counter();

        // ハイパーバイザーレベルでの記録
        self.hypervisor_metrics.record_vmexit(reason, timestamp);

        // ゲストに通知（軽量シグナル）
        self.guest_metrics.signal_vmexit_start(guest_id, event_id);

        // イベント関連性追跡
        self.correlation_engine.start_cross_layer_event(
            event_id, SystemLayer::Hypervisor, timestamp
        );

        event_id
    }

    pub fn record_vmentry(&self, guest_id: u32, event_id: EventId) {
        let timestamp = self.read_cycle_counter();

        // 継続時間計算
        self.correlation_engine.end_cross_layer_event(event_id, timestamp);

        // ゲストに実行復帰通知
        self.guest_metrics.signal_vmentry_complete(guest_id, event_id);

        // リアルタイム分析
        self.realtime_display.update_vmexit_stats();
    }
}
```

### ゲスト側測定エージェント

#### **軽量測定ドライバー**
```c
// guest_measurement_driver.c
#include <linux/module.h>
#include <linux/ktime.h>
#include <linux/hypercall.h>

#define MEASUREMENT_HYPERCALL 0x1000

struct measurement_event {
    u64 event_id;
    u64 guest_timestamp;
    u32 pid;
    u32 tid;
    u32 syscall_nr;
    enum event_type type;
};

struct cross_layer_context {
    struct measurement_event events[MAX_EVENTS];
    atomic_t event_count;
    spinlock_t lock;
    struct timer_list flush_timer;
};

static struct cross_layer_context measurement_ctx;

// システムコール性能測定
asmlinkage long measure_syscall_entry(struct pt_regs *regs) {
    u64 timestamp = ktime_get_ns();
    u32 syscall_nr = regs->orig_ax;

    struct measurement_event event = {
        .guest_timestamp = timestamp,
        .pid = current->pid,
        .tid = current->tid,
        .syscall_nr = syscall_nr,
        .type = SYSCALL_ENTRY
    };

    // ハイパーバイザーに通知
    hypercall_2(MEASUREMENT_HYPERCALL,
               GUEST_EVENT_START, (unsigned long)&event);

    return original_syscall_handler(regs);
}

asmlinkage void measure_syscall_exit(struct pt_regs *regs) {
    u64 timestamp = ktime_get_ns();

    struct measurement_event event = {
        .guest_timestamp = timestamp,
        .pid = current->pid,
        .tid = current->tid,
        .type = SYSCALL_EXIT
    };

    // 終了時刻をハイパーバイザーに通知
    hypercall_2(MEASUREMENT_HYPERCALL,
               GUEST_EVENT_END, (unsigned long)&event);

    original_syscall_exit_handler(regs);
}

// ページフォルト測定
static void measure_page_fault(struct mm_struct *mm,
                              struct vm_area_struct *vma,
                              unsigned long address,
                              unsigned int flags) {
    u64 timestamp = ktime_get_ns();

    struct measurement_event event = {
        .guest_timestamp = timestamp,
        .pid = current->pid,
        .type = PAGE_FAULT,
        .address = address,
        .flags = flags
    };

    // ページフォルト開始をハイパーバイザーに通知
    hypercall_2(MEASUREMENT_HYPERCALL,
               GUEST_PAGE_FAULT, (unsigned long)&event);
}
```

## 🔍 詳細測定シナリオ

### エンドツーエンド応答性能測定

#### **Webアプリケーション応答測定**
```yaml
E2E_Web_Response_Measurement:
  Request_Flow:
    1. Client_Request: "HTTP GET /api/data"
       Timestamp: T0 (network layer)

    2. Guest_Network_Stack: packet processing
       Timestamp: T1 (guest kernel)
       Measurement: packet_receive_latency = T1 - T0

    3. Application_Processing: business logic
       Timestamp: T2 (application layer)
       Measurement: application_latency = T2 - T1

    4. Database_Query: data retrieval
       Timestamp: T3 (database layer)
       VMExit: disk I/O operation
       Hypervisor_Processing: T3h (hypervisor)
       VMEntry: T4 (return to guest)
       Measurement: io_virt_overhead = (T3h + T4) - T3

    5. Response_Generation: result formatting
       Timestamp: T5 (application layer)
       Measurement: processing_latency = T5 - T4

    6. Network_Response: packet transmission
       Timestamp: T6 (guest kernel)
       VMExit: network I/O
       Hypervisor_Network: T6h (hypervisor)
       VMEntry: T7 (return to guest)
       Measurement: net_virt_overhead = (T6h + T7) - T6

    7. Client_Receive: response received
       Timestamp: T8 (network layer)
       Measurement: total_response_time = T8 - T0

  Detailed_Breakdown:
    Pure_Application_Time: (T2-T1) + (T5-T4)
    Virtualization_Overhead: (T3h+T4-T3) + (T6h+T7-T6)
    Network_Latency: (T1-T0) + (T8-T7)
    Total_Overhead_Percentage: virt_overhead / total_time
```

#### **データベーストランザクション測定**
```sql
-- Database Transaction Measurement
DELIMITER $$
CREATE PROCEDURE measure_transaction_performance()
BEGIN
    DECLARE start_time, end_time, commit_time BIGINT;
    DECLARE transaction_id VARCHAR(36);

    -- Generate unique transaction ID
    SET transaction_id = UUID();

    -- Record transaction start
    SET start_time = UNIX_TIMESTAMP(NOW(6)) * 1000000;

    -- Notify hypervisor (custom function)
    SELECT notify_hypervisor('TRANSACTION_START', transaction_id, start_time);

    START TRANSACTION;

    -- Business logic operations
    INSERT INTO orders (customer_id, product_id, quantity, timestamp)
    VALUES (12345, 67890, 10, NOW());

    UPDATE inventory
    SET quantity = quantity - 10
    WHERE product_id = 67890;

    INSERT INTO audit_log (transaction_id, operation, timestamp)
    VALUES (transaction_id, 'inventory_update', NOW());

    -- Record commit time
    SET commit_time = UNIX_TIMESTAMP(NOW(6)) * 1000000;

    COMMIT;

    -- Record transaction end
    SET end_time = UNIX_TIMESTAMP(NOW(6)) * 1000000;

    -- Final notification to hypervisor
    SELECT notify_hypervisor('TRANSACTION_END', transaction_id, end_time);

    -- Return performance metrics
    SELECT
        transaction_id,
        start_time,
        commit_time,
        end_time,
        (commit_time - start_time) as execution_time_us,
        (end_time - commit_time) as commit_overhead_us,
        (end_time - start_time) as total_time_us;
END$$
DELIMITER ;
```

### リアルタイム性能測定

#### **周期タスク応答性測定**
```c
// real_time_measurement.c
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <time.h>
#include <signal.h>
#include <sched.h>
#include <sys/mman.h>

struct rt_measurement {
    struct timespec release_time;
    struct timespec start_time;
    struct timespec completion_time;
    long response_time_us;
    long jitter_us;
    int missed_deadline;
};

static struct rt_measurement measurements[MAX_MEASUREMENTS];
static int measurement_count = 0;
static struct timespec period = {0, 10000000}; // 10ms period

void rt_task_handler(int sig) {
    static struct timespec last_release = {0, 0};
    struct timespec current_time, expected_release;
    long jitter;

    // Record actual start time
    clock_gettime(CLOCK_MONOTONIC, &current_time);

    // Calculate expected release time
    if (last_release.tv_sec == 0) {
        expected_release = current_time;
    } else {
        expected_release.tv_sec = last_release.tv_sec + period.tv_sec;
        expected_release.tv_nsec = last_release.tv_nsec + period.tv_nsec;
        if (expected_release.tv_nsec >= 1000000000) {
            expected_release.tv_sec++;
            expected_release.tv_nsec -= 1000000000;
        }
    }

    // Calculate jitter
    jitter = (current_time.tv_sec - expected_release.tv_sec) * 1000000 +
             (current_time.tv_nsec - expected_release.tv_nsec) / 1000;

    // Record measurement
    if (measurement_count < MAX_MEASUREMENTS) {
        measurements[measurement_count].release_time = expected_release;
        measurements[measurement_count].start_time = current_time;
        measurements[measurement_count].jitter_us = jitter;

        // Simulate real-time work
        perform_rt_work();

        // Record completion
        clock_gettime(CLOCK_MONOTONIC,
                     &measurements[measurement_count].completion_time);

        // Calculate response time
        long response_time =
            (measurements[measurement_count].completion_time.tv_sec -
             measurements[measurement_count].start_time.tv_sec) * 1000000 +
            (measurements[measurement_count].completion_time.tv_nsec -
             measurements[measurement_count].start_time.tv_nsec) / 1000;

        measurements[measurement_count].response_time_us = response_time;

        // Check deadline miss (assuming 10ms deadline)
        measurements[measurement_count].missed_deadline =
            (response_time > 10000) ? 1 : 0;

        measurement_count++;
    }

    last_release = expected_release;
}

void perform_rt_work() {
    // Simulate CPU-intensive work
    volatile int sum = 0;
    for (int i = 0; i < 100000; i++) {
        sum += i * i;
    }

    // Simulate memory access pattern
    static char buffer[4096];
    for (int i = 0; i < 4096; i += 64) {
        buffer[i] = (char)(sum % 256);
    }

    // Simulate I/O operation (may cause VMExit)
    FILE *f = fopen("/dev/null", "w");
    if (f) {
        fwrite(buffer, 1, sizeof(buffer), f);
        fclose(f);
    }
}

void analyze_rt_performance() {
    long total_response = 0;
    long max_response = 0;
    long total_jitter = 0;
    long max_jitter = 0;
    int deadline_misses = 0;

    for (int i = 0; i < measurement_count; i++) {
        total_response += measurements[i].response_time_us;
        if (measurements[i].response_time_us > max_response) {
            max_response = measurements[i].response_time_us;
        }

        total_jitter += abs(measurements[i].jitter_us);
        if (abs(measurements[i].jitter_us) > max_jitter) {
            max_jitter = abs(measurements[i].jitter_us);
        }

        deadline_misses += measurements[i].missed_deadline;
    }

    printf("Real-time Performance Analysis:\n");
    printf("  Average Response Time: %ld μs\n",
           total_response / measurement_count);
    printf("  Maximum Response Time: %ld μs\n", max_response);
    printf("  Average Jitter: %ld μs\n", total_jitter / measurement_count);
    printf("  Maximum Jitter: %ld μs\n", max_jitter);
    printf("  Deadline Misses: %d/%d (%.2f%%)\n",
           deadline_misses, measurement_count,
           100.0 * deadline_misses / measurement_count);
}
```

## 📈 統合分析とレポーティング

### 相関分析エンジン

#### **イベント相関分析**
```python
# correlation_analyzer.py
import numpy as np
import pandas as pd
from datetime import datetime, timedelta
import matplotlib.pyplot as plt
import seaborn as sns

class CrossLayerCorrelationAnalyzer:
    def __init__(self):
        self.events = []
        self.correlations = {}
        self.performance_baselines = {}

    def add_event(self, layer, timestamp, event_type,
                  duration=None, metadata=None):
        event = {
            'layer': layer,
            'timestamp': timestamp,
            'event_type': event_type,
            'duration': duration,
            'metadata': metadata or {}
        }
        self.events.append(event)

    def correlate_events(self, time_window_ms=100):
        """
        指定時間窓内のイベントを相関分析
        """
        df = pd.DataFrame(self.events)
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='us')

        correlations = []

        # アプリケーションレイヤーのイベントを基準に
        app_events = df[df['layer'] == 'application']

        for _, app_event in app_events.iterrows():
            window_start = app_event['timestamp'] - timedelta(milliseconds=time_window_ms/2)
            window_end = app_event['timestamp'] + timedelta(milliseconds=time_window_ms/2)

            related_events = df[
                (df['timestamp'] >= window_start) &
                (df['timestamp'] <= window_end) &
                (df['layer'] != 'application')
            ]

            if len(related_events) > 0:
                correlation = {
                    'app_event': app_event,
                    'related_events': related_events.to_dict('records'),
                    'total_overhead': self.calculate_overhead(related_events),
                    'dominant_bottleneck': self.identify_bottleneck(related_events)
                }
                correlations.append(correlation)

        return correlations

    def calculate_overhead(self, related_events):
        """
        仮想化オーバーヘッドの計算
        """
        hypervisor_time = related_events[
            related_events['layer'] == 'hypervisor'
        ]['duration'].sum()

        total_time = related_events['duration'].sum()

        return {
            'hypervisor_overhead_us': hypervisor_time,
            'total_time_us': total_time,
            'overhead_percentage': (hypervisor_time / total_time) * 100 if total_time > 0 else 0
        }

    def identify_bottleneck(self, related_events):
        """
        主要ボトルネックの特定
        """
        bottlenecks = related_events.groupby('event_type')['duration'].agg([
            'count', 'mean', 'sum'
        ]).sort_values('sum', ascending=False)

        if len(bottlenecks) > 0:
            return {
                'type': bottlenecks.index[0],
                'total_time': bottlenecks.iloc[0]['sum'],
                'frequency': bottlenecks.iloc[0]['count'],
                'average_duration': bottlenecks.iloc[0]['mean']
            }
        return None

    def generate_performance_report(self, output_file='performance_report.html'):
        """
        統合性能レポートの生成
        """
        correlations = self.correlate_events()

        # データ集約
        overhead_stats = [c['total_overhead'] for c in correlations]
        bottleneck_types = [c['dominant_bottleneck']['type']
                           for c in correlations if c['dominant_bottleneck']]

        # 統計分析
        avg_overhead = np.mean([o['overhead_percentage'] for o in overhead_stats])
        max_overhead = np.max([o['overhead_percentage'] for o in overhead_stats])

        # 可視化
        fig, axes = plt.subplots(2, 2, figsize=(15, 10))

        # オーバーヘッド分布
        overhead_percentages = [o['overhead_percentage'] for o in overhead_stats]
        axes[0, 0].hist(overhead_percentages, bins=20, alpha=0.7)
        axes[0, 0].set_title('Virtualization Overhead Distribution')
        axes[0, 0].set_xlabel('Overhead Percentage')
        axes[0, 0].set_ylabel('Frequency')

        # ボトルネック分析
        bottleneck_counts = pd.Series(bottleneck_types).value_counts()
        axes[0, 1].pie(bottleneck_counts.values, labels=bottleneck_counts.index, autopct='%1.1f%%')
        axes[0, 1].set_title('Dominant Bottlenecks')

        # 時間軸パフォーマンス
        df = pd.DataFrame(self.events)
        df['timestamp'] = pd.to_datetime(df['timestamp'], unit='us')
        performance_timeline = df.groupby([
            pd.Grouper(key='timestamp', freq='1min'),
            'layer'
        ])['duration'].mean().reset_index()

        for layer in df['layer'].unique():
            layer_data = performance_timeline[performance_timeline['layer'] == layer]
            axes[1, 0].plot(layer_data['timestamp'], layer_data['duration'],
                           label=layer, marker='o')

        axes[1, 0].set_title('Performance Timeline by Layer')
        axes[1, 0].set_xlabel('Time')
        axes[1, 0].set_ylabel('Average Duration (μs)')
        axes[1, 0].legend()
        axes[1, 0].tick_params(axis='x', rotation=45)

        # レイヤー別性能比較
        layer_performance = df.groupby('layer')['duration'].agg(['mean', 'std'])
        axes[1, 1].bar(layer_performance.index, layer_performance['mean'],
                      yerr=layer_performance['std'], capsize=5)
        axes[1, 1].set_title('Performance by Layer')
        axes[1, 1].set_xlabel('System Layer')
        axes[1, 1].set_ylabel('Average Duration (μs)')

        plt.tight_layout()
        plt.savefig('performance_analysis.png', dpi=300, bbox_inches='tight')

        # HTMLレポート生成
        html_report = f"""
        <html>
        <head><title>Cross-Layer Performance Analysis Report</title></head>
        <body>
        <h1>Cross-Layer Performance Analysis Report</h1>
        <h2>Executive Summary</h2>
        <ul>
        <li>Average Virtualization Overhead: {avg_overhead:.2f}%</li>
        <li>Maximum Virtualization Overhead: {max_overhead:.2f}%</li>
        <li>Total Events Analyzed: {len(self.events)}</li>
        <li>Correlated Event Groups: {len(correlations)}</li>
        </ul>

        <h2>Performance Visualization</h2>
        <img src="performance_analysis.png" alt="Performance Analysis Charts">

        <h2>Detailed Analysis</h2>
        <p>This report provides comprehensive analysis of cross-layer performance
           in the Type-1 hypervisor environment.</p>
        </body>
        </html>
        """

        with open(output_file, 'w') as f:
            f.write(html_report)

        return {
            'average_overhead': avg_overhead,
            'maximum_overhead': max_overhead,
            'total_events': len(self.events),
            'correlations_found': len(correlations),
            'report_file': output_file
        }

# 使用例
if __name__ == "__main__":
    analyzer = CrossLayerCorrelationAnalyzer()

    # サンプルデータの追加
    # 実際の測定データをここに読み込み

    # 分析実行
    report = analyzer.generate_performance_report()
    print(f"Analysis complete. Report generated: {report}")
```

### 自動最適化提案

#### **最適化推奨エンジン**
```python
# optimization_recommender.py
class PerformanceOptimizationRecommender:
    def __init__(self, correlation_analyzer):
        self.analyzer = correlation_analyzer
        self.optimization_rules = self.load_optimization_rules()

    def load_optimization_rules(self):
        return {
            'high_vmexit_frequency': {
                'threshold': 1000,  # VMExits per second
                'recommendations': [
                    'Enable SR-IOV for network devices',
                    'Increase stage-2 TLB size',
                    'Optimize MMIO handling',
                    'Consider CPU pinning'
                ]
            },
            'high_memory_overhead': {
                'threshold': 20,  # percentage
                'recommendations': [
                    'Enable large page support',
                    'Optimize memory ballooning',
                    'Review memory overcommit ratio',
                    'Consider NUMA topology optimization'
                ]
            },
            'io_bottleneck': {
                'threshold': 100,  # ms average latency
                'recommendations': [
                    'Enable virtio for storage devices',
                    'Implement I/O scheduling optimization',
                    'Consider device passthrough',
                    'Optimize storage queue depth'
                ]
            }
        }

    def analyze_and_recommend(self):
        performance_data = self.analyzer.correlate_events()
        recommendations = []

        # VMExit頻度チェック
        vmexit_frequency = self.calculate_vmexit_frequency(performance_data)
        if vmexit_frequency > self.optimization_rules['high_vmexit_frequency']['threshold']:
            recommendations.extend(
                self.optimization_rules['high_vmexit_frequency']['recommendations']
            )

        # メモリオーバーヘッドチェック
        memory_overhead = self.calculate_memory_overhead(performance_data)
        if memory_overhead > self.optimization_rules['high_memory_overhead']['threshold']:
            recommendations.extend(
                self.optimization_rules['high_memory_overhead']['recommendations']
            )

        # I/Oボトルネックチェック
        io_latency = self.calculate_io_latency(performance_data)
        if io_latency > self.optimization_rules['io_bottleneck']['threshold']:
            recommendations.extend(
                self.optimization_rules['io_bottleneck']['recommendations']
            )

        return {
            'recommendations': list(set(recommendations)),  # 重複除去
            'analysis_data': {
                'vmexit_frequency': vmexit_frequency,
                'memory_overhead': memory_overhead,
                'io_latency': io_latency
            }
        }
```

<function_calls>
<invoke name="TodoWrite">
<parameter name="todos">[{"content": "Design comprehensive performance evaluation framework for hypervisor systems", "status": "completed", "activeForm": "Designing performance evaluation framework"}, {"content": "Create resource allocation design guidelines", "status": "completed", "activeForm": "Creating resource allocation guidelines"}, {"content": "Establish cross-layer performance measurement methodology", "status": "completed", "activeForm": "Establishing cross-layer measurement methodology"}]