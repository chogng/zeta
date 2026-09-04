use super::ProcessResourceDemand;
use super::ProcessResourceMetrics;
use super::ProcessResourceRequest;
use super::ProcessResourceSampleIntervals;
use super::ProcessResourceTargets;
use super::ProcessResourcesSampler;
use super::ProcessResourcesSource;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use std::time::Instant;

#[test]
fn demand_uses_no_timer_when_disabled_and_slower_status_line_sampling() {
    let intervals = ProcessResourceSampleIntervals::default();

    assert_eq!(
        ProcessResourceDemand::Disabled.sample_interval(intervals),
        None
    );
    assert_eq!(
        ProcessResourceDemand::StatusLine(ProcessResourceMetrics::Memory)
            .sample_interval(intervals),
        Some(Duration::from_secs(2))
    );
    assert_eq!(
        ProcessResourceDemand::Processes.sample_interval(intervals),
        Some(Duration::from_secs(1))
    );
}

#[test]
fn sampler_reads_tui_memory_and_collects_cpu_after_the_first_interval() {
    let request = ProcessResourceRequest {
        revision: 1,
        cpu_cycle: 1,
        demand: ProcessResourceDemand::Processes,
    };
    let mut sampler = ProcessResourcesSampler::new(
        ProcessResourceTargets::Tui,
        ProcessResourceMetrics::MemoryAndCpu,
        request.cpu_cycle,
    );
    let first = sampler.sample(request, Instant::now());
    let first = first.tui.unwrap();
    assert!(first.resident_bytes.unwrap() > 0);
    assert_eq!(first.cpu_tenths_percent, None);

    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    let second = sampler.sample(request, Instant::now()).tui.unwrap();
    assert!(second.cpu_tenths_percent.is_some());
    assert!(second.cpu_tenths_percent.unwrap() <= 1_000);
}

#[test]
fn source_waits_for_demand_samples_only_requested_metrics_and_joins_promptly() {
    let stop = Arc::new(AtomicBool::new(false));
    let readings = Arc::new(Mutex::new(Vec::new()));
    let emitted = Arc::clone(&readings);
    let mut source = ProcessResourcesSource::start_with_intervals(
        Arc::clone(&stop),
        ProcessResourceTargets::Tui,
        ProcessResourceSampleIntervals {
            status_line: Duration::from_millis(10),
            processes: Duration::from_millis(10),
        },
        move |reading| {
            emitted.lock().unwrap().push(reading);
            true
        },
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(25));
    assert!(readings.lock().unwrap().is_empty());

    let memory_request = ProcessResourceRequest {
        revision: 1,
        cpu_cycle: 0,
        demand: ProcessResourceDemand::StatusLine(ProcessResourceMetrics::Memory),
    };
    source.set_request(memory_request);
    let deadline = Instant::now() + Duration::from_secs(2);
    while readings.lock().unwrap().is_empty() && Instant::now() < deadline {
        std::thread::yield_now();
    }
    let first = readings.lock().unwrap()[0].clone();
    assert_eq!(first.request, memory_request);
    assert!(first.tui.unwrap().resident_bytes.is_some());

    let cpu_request = ProcessResourceRequest {
        revision: 2,
        cpu_cycle: 1,
        demand: ProcessResourceDemand::StatusLine(ProcessResourceMetrics::Cpu),
    };
    source.set_request(cpu_request);
    let deadline = Instant::now() + Duration::from_secs(2);
    while readings
        .lock()
        .unwrap()
        .iter()
        .all(|reading| reading.request != cpu_request)
        && Instant::now() < deadline
    {
        std::thread::yield_now();
    }
    let current_readings = readings.lock().unwrap();
    let cpu = current_readings
        .iter()
        .find(|reading| reading.request == cpu_request)
        .unwrap()
        .tui
        .as_ref()
        .unwrap();
    assert_eq!(cpu.resident_bytes, None);
    drop(current_readings);

    source.set_request(ProcessResourceRequest {
        revision: 3,
        cpu_cycle: 1,
        demand: ProcessResourceDemand::Disabled,
    });
    std::thread::sleep(Duration::from_millis(30));
    let stopped_at = readings.lock().unwrap().len();
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(readings.lock().unwrap().len(), stopped_at);

    let restarted_cpu_request = ProcessResourceRequest {
        revision: 4,
        cpu_cycle: 2,
        demand: ProcessResourceDemand::StatusLine(ProcessResourceMetrics::Cpu),
    };
    source.set_request(restarted_cpu_request);
    let deadline = Instant::now() + Duration::from_secs(2);
    while readings
        .lock()
        .unwrap()
        .iter()
        .all(|reading| reading.request != restarted_cpu_request)
        && Instant::now() < deadline
    {
        std::thread::yield_now();
    }
    let restarted = readings
        .lock()
        .unwrap()
        .iter()
        .find(|reading| reading.request == restarted_cpu_request)
        .unwrap()
        .tui
        .as_ref()
        .unwrap()
        .cpu_tenths_percent;
    assert_eq!(restarted, None);

    let started = Instant::now();
    source.join().unwrap();

    assert!(started.elapsed() < Duration::from_millis(250));
}
