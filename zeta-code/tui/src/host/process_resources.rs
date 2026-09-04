use std::io;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;
use std::time::Instant;
use sysinfo::Pid;
use sysinfo::ProcessRefreshKind;
use sysinfo::ProcessesToUpdate;
use sysinfo::System;
use sysinfo::get_current_pid;

const STATUS_LINE_SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
const PROCESSES_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
struct ProcessResourceSampleIntervals {
    status_line: Duration,
    processes: Duration,
}

impl Default for ProcessResourceSampleIntervals {
    fn default() -> Self {
        Self {
            status_line: STATUS_LINE_SAMPLE_INTERVAL,
            processes: PROCESSES_SAMPLE_INTERVAL,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessResourceTargets {
    Tui,
    TuiAndAppServer(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessResourceMetrics {
    Memory,
    Cpu,
    MemoryAndCpu,
}

impl ProcessResourceMetrics {
    pub(crate) const fn includes_memory(self) -> bool {
        matches!(self, Self::Memory | Self::MemoryAndCpu)
    }

    pub(crate) const fn includes_cpu(self) -> bool {
        matches!(self, Self::Cpu | Self::MemoryAndCpu)
    }

    pub(crate) const fn union(self, other: Self) -> Self {
        match (self, other) {
            (Self::Memory, Self::Memory) => Self::Memory,
            (Self::Cpu, Self::Cpu) => Self::Cpu,
            _ => Self::MemoryAndCpu,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ProcessResourceDemand {
    #[default]
    Disabled,
    StatusLine(ProcessResourceMetrics),
    Processes,
}

impl ProcessResourceDemand {
    pub(crate) const fn metrics(self) -> Option<ProcessResourceMetrics> {
        match self {
            Self::Disabled => None,
            Self::StatusLine(metrics) => Some(metrics),
            Self::Processes => Some(ProcessResourceMetrics::MemoryAndCpu),
        }
    }

    const fn sample_interval(self, intervals: ProcessResourceSampleIntervals) -> Option<Duration> {
        match self {
            Self::Disabled => None,
            Self::StatusLine(_) => Some(intervals.status_line),
            Self::Processes => Some(intervals.processes),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ProcessResourceRequest {
    pub(crate) revision: u64,
    pub(crate) cpu_cycle: u64,
    pub(crate) demand: ProcessResourceDemand,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProcessResourceUsage {
    pub(crate) resident_bytes: Option<u64>,
    pub(crate) cpu_tenths_percent: Option<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProcessResourcesReading {
    pub(crate) request: ProcessResourceRequest,
    pub(crate) tui: Result<ProcessResourceUsage, String>,
    pub(crate) app_server: Option<Result<ProcessResourceUsage, String>>,
    pub(crate) sampled_at: Instant,
}

pub(crate) struct ProcessResourcesSource {
    stop: Arc<AtomicBool>,
    control: Arc<ProcessResourcesControl>,
    task: Option<JoinHandle<()>>,
}

#[derive(Default)]
struct ProcessResourcesControl {
    request: Mutex<ProcessResourceRequest>,
    changed: Condvar,
}

impl ProcessResourcesSource {
    pub(crate) fn start(
        stop: Arc<AtomicBool>,
        targets: ProcessResourceTargets,
        emit: impl FnMut(ProcessResourcesReading) -> bool + Send + 'static,
    ) -> Result<Self, io::Error> {
        Self::start_with_intervals(
            stop,
            targets,
            ProcessResourceSampleIntervals::default(),
            emit,
        )
    }

    fn start_with_intervals(
        stop: Arc<AtomicBool>,
        targets: ProcessResourceTargets,
        intervals: ProcessResourceSampleIntervals,
        mut emit: impl FnMut(ProcessResourcesReading) -> bool + Send + 'static,
    ) -> Result<Self, io::Error> {
        let task_stop = Arc::clone(&stop);
        let control = Arc::new(ProcessResourcesControl::default());
        let task_control = Arc::clone(&control);
        let task = thread::Builder::new()
            .name("zeta-tui-process-resources".into())
            .spawn(move || {
                let mut sampler = None;
                loop {
                    let request = {
                        let mut request = task_control.request.lock().unwrap();
                        while matches!(request.demand, ProcessResourceDemand::Disabled)
                            && !task_stop.load(Ordering::Acquire)
                        {
                            sampler = None;
                            request = task_control.changed.wait(request).unwrap();
                        }
                        if task_stop.load(Ordering::Acquire) {
                            return;
                        }
                        *request
                    };
                    let metrics = request
                        .demand
                        .metrics()
                        .expect("an active resource demand has metrics");
                    let sampler = sampler.get_or_insert_with(|| {
                        ProcessResourcesSampler::new(targets, metrics, request.cpu_cycle)
                    });
                    sampler.set_request(request);
                    let reading = sampler.sample(request, Instant::now());
                    if !emit(reading) {
                        return;
                    }
                    let interval = request
                        .demand
                        .sample_interval(intervals)
                        .expect("an active resource demand has an interval");
                    let current = task_control.request.lock().unwrap();
                    let _ = task_control
                        .changed
                        .wait_timeout_while(current, interval, |current| {
                            *current == request && !task_stop.load(Ordering::Acquire)
                        })
                        .unwrap();
                }
            })?;
        Ok(Self {
            stop,
            control,
            task: Some(task),
        })
    }

    pub(crate) fn set_request(&self, request: ProcessResourceRequest) {
        let mut current = self.control.request.lock().unwrap();
        if *current == request {
            return;
        }
        *current = request;
        self.control.changed.notify_one();
    }

    pub(crate) fn join(&mut self) -> Result<(), io::Error> {
        self.stop.store(true, Ordering::Release);
        let Some(task) = self.task.take() else {
            return Ok(());
        };
        self.control.changed.notify_one();
        task.join()
            .map_err(|_| io::Error::other("process resource source panicked"))
    }
}

impl Drop for ProcessResourcesSource {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.join();
    }
}

struct ProcessResourcesSampler {
    state: Result<SamplerState, String>,
    targets: ProcessResourceTargets,
    metrics: ProcessResourceMetrics,
    cpu_cycle: u64,
}

struct SamplerState {
    system: System,
    tui_pid: Pid,
    app_server_pid: Option<Pid>,
    logical_processors: usize,
    cpu_ready: bool,
}

impl ProcessResourcesSampler {
    fn new(
        targets: ProcessResourceTargets,
        metrics: ProcessResourceMetrics,
        cpu_cycle: u64,
    ) -> Self {
        let state = if sysinfo::IS_SUPPORTED_SYSTEM {
            get_current_pid()
                .map(|tui_pid| SamplerState {
                    system: System::new(),
                    tui_pid,
                    app_server_pid: match targets {
                        ProcessResourceTargets::Tui => None,
                        ProcessResourceTargets::TuiAndAppServer(process_id) => {
                            Some(Pid::from_u32(process_id))
                        }
                    },
                    logical_processors: thread::available_parallelism()
                        .map(usize::from)
                        .unwrap_or(1),
                    cpu_ready: false,
                })
                .map_err(|error| format!("could not identify the TUI process: {error}"))
        } else {
            Err("process resources are unavailable on this operating system".into())
        };
        Self {
            state,
            targets,
            metrics,
            cpu_cycle,
        }
    }

    fn set_request(&mut self, request: ProcessResourceRequest) {
        let metrics = request
            .demand
            .metrics()
            .expect("an active resource demand has metrics");
        if self.cpu_cycle != request.cpu_cycle
            || !self.metrics.includes_cpu()
            || !metrics.includes_cpu()
        {
            if let Ok(state) = self.state.as_mut() {
                state.cpu_ready = false;
            }
        }
        self.metrics = metrics;
        self.cpu_cycle = request.cpu_cycle;
    }

    fn sample(
        &mut self,
        request: ProcessResourceRequest,
        sampled_at: Instant,
    ) -> ProcessResourcesReading {
        let state = match self.state.as_mut() {
            Ok(state) => state,
            Err(error) => {
                return ProcessResourcesReading {
                    request,
                    tui: Err(error.clone()),
                    app_server: matches!(self.targets, ProcessResourceTargets::TuiAndAppServer(_))
                        .then(|| Err(error.clone())),
                    sampled_at,
                };
            }
        };
        let mut pids = vec![state.tui_pid];
        if let Some(pid) = state.app_server_pid {
            pids.push(pid);
        }
        let mut refresh = ProcessRefreshKind::nothing();
        if self.metrics.includes_memory() {
            refresh = refresh.with_memory();
        }
        if self.metrics.includes_cpu() {
            refresh = refresh.with_cpu();
        }
        state
            .system
            .refresh_processes_specifics(ProcessesToUpdate::Some(&pids), true, refresh);
        let tui = process_usage(
            &state.system,
            state.tui_pid,
            "TUI",
            self.metrics,
            state.cpu_ready,
            state.logical_processors,
        );
        let app_server = state.app_server_pid.map(|pid| {
            process_usage(
                &state.system,
                pid,
                "App Server",
                self.metrics,
                state.cpu_ready,
                state.logical_processors,
            )
        });
        state.cpu_ready = self.metrics.includes_cpu();
        ProcessResourcesReading {
            request,
            tui,
            app_server,
            sampled_at,
        }
    }
}

fn process_usage(
    system: &System,
    pid: Pid,
    label: &str,
    metrics: ProcessResourceMetrics,
    cpu_ready: bool,
    logical_processors: usize,
) -> Result<ProcessResourceUsage, String> {
    let process = system
        .process(pid)
        .ok_or_else(|| format!("{label} process resources are unavailable"))?;
    let resident_bytes = if metrics.includes_memory() {
        let resident_bytes = process.memory();
        if resident_bytes == 0 {
            return Err(format!("{label} resident memory is unavailable"));
        }
        Some(resident_bytes)
    } else {
        None
    };
    let cpu_tenths_percent = (metrics.includes_cpu() && cpu_ready).then(|| {
        let normalized = process.cpu_usage() / logical_processors.max(1) as f32;
        (normalized.clamp(0.0, 100.0) * 10.0).round() as u16
    });
    Ok(ProcessResourceUsage {
        resident_bytes,
        cpu_tenths_percent,
    })
}

#[cfg(test)]
#[path = "process_resources_tests.rs"]
mod tests;
