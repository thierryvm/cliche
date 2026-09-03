//! Latency instrumentation for the capture pipeline.
//!
//! Lot 1 has to prove that "shortcut pressed -> overlay painted" holds under
//! 150 ms. Without an instrument that number is an impression, so this module
//! is the piece the whole verdict rests on: if it lies, the lot lies.
//!
//! Everything here is `std` only. The file is deliberately NOT called `chrono`
//! or anything close to it: no date, no wall clock, no calendar is involved.
//!
//! Two rules shape the design:
//!
//! - **`Instant`, never `SystemTime`.** A wall clock jumps - NTP correction,
//!   daylight saving, a user fixing the date - and a jump backwards makes a
//!   measured duration negative, which `SystemTime::duration_since` reports as
//!   an error and naive code turns into a wrapped, absurd value. `Instant` is
//!   monotonic by contract, which is the only property a stopwatch needs.
//! - **Nothing here may panic.** These calls run inside the global shortcut
//!   handler and inside the webview's paint acknowledgement. A panic on either
//!   path takes the application down, and losing the app to fix a diagnostic is
//!   a bad trade. Hence: no `unwrap` on a real path, saturating arithmetic, and
//!   a poisoned lock that is recovered rather than propagated.

use std::collections::VecDeque;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How many finished runs are kept. A capture tool stays open for days; an
/// unbounded history would be a slow leak in a process nobody restarts. 200 is
/// far above the 20 runs the measurement protocol asks for, so the cap never
/// bites during a measurement session - it only stops the drift.
const MAX_RUNS: usize = 200;

/// How many steps one run may record. Bounds both memory and the duplicate
/// scan below if a caller ever marks inside a loop. The real pipeline has about
/// six steps.
const MAX_STEPS_PER_RUN: usize = 64;

/// Expected number of steps per run, used to size the buffer once at
/// `begin_run` so that `mark` itself never allocates.
const TYPICAL_STEPS_PER_RUN: usize = 8;

/// One recorded step: a label, and how long after the start of its run it
/// happened.
///
/// The offset from the run start is stored rather than the gap from the
/// previous step, because it is what `Instant::elapsed` gives directly. Gaps
/// are derived at report time by `step_gaps`, where a single non-monotonic
/// pair costs one zero instead of a wrapped subtraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Step {
    label: &'static str,
    at: Duration,
}

/// A run being measured right now.
#[derive(Debug)]
struct OpenRun {
    started: Instant,
    steps: Vec<Step>,
}

#[derive(Debug, Default)]
struct State {
    open: Option<OpenRun>,
    finished: VecDeque<Vec<Step>>,
    /// Runs started and never finished, plus finished runs that recorded
    /// nothing. Surfaced in the report: a high count means the pipeline is
    /// bailing out somewhere, which is exactly the kind of thing a latency
    /// median would otherwise hide.
    discarded_runs: usize,
    /// Marks that could not be recorded: no run open, duplicate label, or the
    /// per-run cap reached. Also surfaced - a silently dropped mark would make
    /// a step look faster than it is.
    ignored_marks: usize,
}

/// The instrument. Meant to be handed to `app.manage(Timings::new())` and
/// borrowed from anywhere as `State<Timings>`.
///
/// **Sharing: `Mutex`, deliberately.**
/// The global shortcut handler and the webview's paint acknowledgement arrive
/// on different threads, so the state must be `Send + Sync`; `Mutex<State>`
/// is, without the caller needing `&mut`.
///
/// - Not atomics: the state is a compound value (an open run, a history, two
///   counters) and a mark mutates several parts at once. Field-by-field atomics
///   would let a report observe half an update.
/// - Not `RwLock`: on the hot path *every* operation writes. A read/write lock
///   is heavier than a mutex and would only pay off if reads dominated, whereas
///   reading happens once, at the end, when a report is asked for.
/// - Not a channel to a collector thread: that would move the cost off the hot
///   path, but it adds a thread and makes "ask for the report now" asynchronous
///   for a lock that is held for a few dozen nanoseconds.
///
/// **Cost.** On the hot path a mark is: one uncontended lock, one
/// `Instant::now()`, a scan of at most a handful of `&'static str` labels, and
/// a push into a buffer sized once at `begin_run`. No allocation, no
/// formatting, no I/O - labels are `&'static str` precisely so that recording
/// one never touches the heap. That is nanoseconds against a 150 ms budget.
/// An instrument that cost 10 ms would be measuring itself.
/// This reasoning is bounded by a test (`the_instrument_costs_far_less_...`),
/// which proves a ceiling; it is not a measurement of the true cost.
#[derive(Debug, Default)]
pub struct Timings {
    inner: Mutex<State>,
}

impl Timings {
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the lock, recovering it if another thread panicked while holding
    /// it. The guarded value is plain measurement data with no invariant to
    /// uphold, so the worst a poisoned lock can hide is one dubious sample in a
    /// diagnostic. `unwrap()` here would turn a panic in one thread into a dead
    /// application - the opposite of what an instrument is for.
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Opens a run. Call this first thing in the shortcut handler.
    ///
    /// If a run is already open it is dropped, not finished: the shortcut was
    /// pressed twice and the first overlay never painted. Folding a half run
    /// into the aggregate would make the pipeline look faster than it is.
    pub fn begin_run(&self) {
        // Read the clock BEFORE reaching for the lock. Acquiring it is cheap
        // but not free, and whatever it costs would otherwise be counted as
        // part of the first step - the instrument would be measuring itself.
        let started = Instant::now();
        let mut state = self.state();

        if state.open.is_some() {
            state.discarded_runs += 1;
        }

        state.open = Some(OpenRun {
            started,
            // Sized once, here, so that `mark` never allocates. This runs at
            // shortcut press, before the capture, and it is one small
            // allocation.
            steps: Vec::with_capacity(TYPICAL_STEPS_PER_RUN),
        });
    }

    /// Timestamps a step of the run in progress.
    ///
    /// Ignored, and counted, when there is no open run (an acknowledgement
    /// arriving after Escape closed the overlay), when the label was already
    /// recorded in this run (two paint acknowledgements for one capture), or
    /// when the per-run cap is reached. Keeping a duplicate would average two
    /// different things under one name; dropping it silently would hide a bug
    /// in the caller. So: keep the first, count the second, say it in the
    /// report.
    pub fn mark(&self, label: &'static str) {
        // Same reason as `begin_run`: the timestamp is taken before the lock,
        // so a contended lock lengthens no step. `saturating_duration_since`
        // and not `duration_since`, which panics when the earlier instant is
        // later - impossible with a monotonic clock, and not worth an
        // application crash if a platform ever disagrees.
        let now = Instant::now();
        let mut state = self.state();

        let recorded = match state.open.as_mut() {
            Some(run) => {
                let duplicate = run.steps.iter().any(|step| step.label == label);
                if duplicate || run.steps.len() >= MAX_STEPS_PER_RUN {
                    false
                } else {
                    let at = now.saturating_duration_since(run.started);
                    run.steps.push(Step { label, at });
                    true
                }
            }
            None => false,
        };

        if !recorded {
            state.ignored_marks += 1;
        }
    }

    /// Closes the run in progress and files it.
    ///
    /// A no-op when no run is open. A run that recorded no step is discarded
    /// rather than filed: its total would be zero and would drag every
    /// aggregate down with a number that measures nothing.
    pub fn finish_run(&self) {
        let mut state = self.state();
        let Some(run) = state.open.take() else {
            return;
        };

        if run.steps.is_empty() {
            state.discarded_runs += 1;
            return;
        }

        if state.finished.len() == MAX_RUNS {
            state.finished.pop_front();
        }
        state.finished.push_back(run.steps);
    }

    /// Drops the run in progress without filing it. For Escape: the user
    /// cancelled, there is no latency to report.
    pub fn abandon_run(&self) {
        let mut state = self.state();
        if state.open.take().is_some() {
            state.discarded_runs += 1;
        }
    }

    /// Aggregates every finished run. Never panics, including with no run at
    /// all - the report then says so, which is a result, not an error.
    pub fn report(&self) -> Report {
        let state = self.state();
        aggregate(
            state.finished.iter().map(Vec::as_slice),
            state.discarded_runs,
            state.ignored_marks,
        )
    }

    /// Files a ready-made run, bypassing the clock.
    ///
    /// Test seam. Aggregation has to be checked against values known by hand,
    /// and values obtained by actually sleeping are neither known nor stable.
    /// The offsets are cumulative, exactly as `mark` would have recorded them.
    #[cfg(test)]
    fn file_run(&self, steps: &[(&'static str, Duration)]) {
        let steps = steps
            .iter()
            .map(|&(label, at)| Step { label, at })
            .collect();
        self.state().finished.push_back(steps);
    }
}

/// Turns per-run offsets into per-step gaps: how long each step itself took.
///
/// `saturating_sub`, not `-`. `Instant` is monotonic by contract, so offsets
/// should never decrease; should that contract ever bend on some platform, a
/// plain subtraction panics in debug and wraps to ~584 years in release. A zero
/// is wrong by a hair, a wrapped value is wrong by an era, and neither may take
/// the application down.
fn step_gaps(steps: &[Step]) -> Vec<(&'static str, Duration)> {
    let mut previous = Duration::ZERO;
    let mut gaps = Vec::with_capacity(steps.len());

    for step in steps {
        gaps.push((step.label, step.at.saturating_sub(previous)));
        previous = step.at;
    }

    gaps
}

/// Median: middle value on an odd count, mean of the two middle values on an
/// even count. The definition everyone expects, and deliberately NOT the
/// `percentile(50)` below - see there for why the two differ.
///
/// `None` on an empty series: there is no median of nothing, and returning zero
/// would be a fabricated measurement.
fn median(samples: &[Duration]) -> Option<Duration> {
    if samples.is_empty() {
        return None;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_unstable();

    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 1 {
        return Some(sorted[middle]);
    }

    // `saturating_add` rather than `+`: adding two durations panics on
    // overflow. Unreachable with latencies, but this is a no-panic module and
    // the saturating form costs nothing.
    Some(sorted[middle - 1].saturating_add(sorted[middle]) / 2)
}

/// Percentile by the **nearest-rank method, inclusive**: with `n` sorted
/// samples, take the one at rank `ceil(p * n / 100)`, counting from 1.
///
/// "p95" has no single definition, and on 20 samples the usual candidates
/// disagree: nearest rank gives the 19th sample, linear interpolation gives a
/// value between the 19th and the 20th that *no run ever produced*. Nearest
/// rank is chosen here because this instrument arbitrates a latency budget: the
/// number it prints has to be a latency that actually happened, one that can be
/// pointed at in the raw samples. An interpolated figure cannot.
///
/// The consequence has to be said out loud rather than discovered later: at
/// n = 10, `ceil(0.95 * 10) = 10`, so the p95 IS the maximum and carries no
/// more information than "the worst of ten". That is precisely why the
/// measurement protocol wants at least 20 runs.
///
/// The rank is computed in integer arithmetic. `0.95 * 20` in binary floating
/// point is not exactly 19, and a percentile whose result depends on which side
/// of an ulp the rounding lands is not an instrument.
fn percentile(samples: &[Duration], p: u8) -> Option<Duration> {
    if samples.is_empty() {
        return None;
    }

    let mut sorted = samples.to_vec();
    sorted.sort_unstable();

    // At least 1 (p = 0 asks for the smallest sample), at most n.
    let rank = (usize::from(p) * sorted.len())
        .div_ceil(100)
        .clamp(1, sorted.len());

    Some(sorted[rank - 1])
}

/// Statistics for one step, across every finished run that recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepStats {
    pub label: &'static str,
    /// Time this step itself took, not the offset from the start of the run.
    pub median: Duration,
    pub p95: Duration,
    /// Runs that recorded this step. Below the run count when a step is
    /// conditional - worth seeing before trusting its median.
    pub samples: usize,
}

/// What the instrument has to say. Plain data, no lock held.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Finished runs the figures are computed on.
    pub runs: usize,
    /// Steps in the order they were first seen, which is the order of the
    /// pipeline.
    pub steps: Vec<StepStats>,
    /// Whole run, shortcut press to last step. This is the figure the 150 ms
    /// budget is about.
    pub total_median: Duration,
    pub total_p95: Duration,
    pub discarded_runs: usize,
    pub ignored_marks: usize,
}

fn aggregate<'a>(
    runs: impl Iterator<Item = &'a [Step]>,
    discarded_runs: usize,
    ignored_marks: usize,
) -> Report {
    // Insertion-ordered: the report must read in pipeline order, so a map that
    // sorts or hashes its keys would be the wrong container. The number of
    // distinct labels is a handful, so the linear lookup is cheaper than the
    // hashing it replaces.
    let mut per_step: Vec<(&'static str, Vec<Duration>)> = Vec::new();
    let mut totals: Vec<Duration> = Vec::new();
    let mut run_count = 0usize;

    for steps in runs {
        run_count += 1;

        // The last offset is the whole run: start of the shortcut handler to
        // the final step.
        if let Some(last) = steps.last() {
            totals.push(last.at);
        }

        for (label, gap) in step_gaps(steps) {
            // Looked up by index, in its own statement, rather than by holding
            // an iterator across the `match`: the temporary from `iter_mut()`
            // would keep the borrow alive into the arm that pushes.
            let known = per_step.iter().position(|(seen, _)| *seen == label);

            match known {
                Some(index) => {
                    if let Some((_, samples)) = per_step.get_mut(index) {
                        samples.push(gap);
                    }
                }
                None => per_step.push((label, vec![gap])),
            }
        }
    }

    let steps = per_step
        .into_iter()
        .map(|(label, samples)| StepStats {
            label,
            // `unwrap_or_default` and not `unwrap`: a label only exists here
            // because a sample was pushed for it, so the `None` branch is
            // unreachable - but "unreachable" is not a reason to panic in a
            // shortcut handler. A zero would be visible in the report.
            median: median(&samples).unwrap_or_default(),
            p95: percentile(&samples, 95).unwrap_or_default(),
            samples: samples.len(),
        })
        .collect();

    Report {
        runs: run_count,
        steps,
        total_median: median(&totals).unwrap_or_default(),
        total_p95: percentile(&totals, 95).unwrap_or_default(),
        discarded_runs,
        ignored_marks,
    }
}

/// Milliseconds with one decimal. Microseconds would be noise on a 150 ms
/// budget, whole milliseconds would hide the difference between 0.4 and 1.4.
fn millis(duration: Duration) -> String {
    format!("{:.1} ms", duration.as_secs_f64() * 1000.0)
}

impl Report {
    /// Renders the report as human-readable lines.
    ///
    /// Split from printing so it can be unit-tested, like `summarize` in
    /// `displays.rs`. ASCII only, same reason: the Windows console is not
    /// reliably UTF-8 and a mangled diagnostic is worse than a plain one.
    pub fn lines(&self) -> Vec<String> {
        if self.runs == 0 {
            // Not an empty output and not an error: "nothing was measured" is
            // itself a finding, and the most likely one to be misread as "the
            // measurement passed".
            let mut lines = vec!["timing: no finished run to report on".to_owned()];
            lines.extend(self.anomaly_lines());
            return lines;
        }

        let mut lines = Vec::with_capacity(self.steps.len() + 3);
        lines.push(format!("timing report over {} run(s)", self.runs));

        for (index, step) in self.steps.iter().enumerate() {
            lines.push(format!(
                "  #{rank} {label:<20} median {median:>9}  p95 {p95:>9}  ({samples} sample(s))",
                rank = index + 1,
                label = step.label,
                median = millis(step.median),
                p95 = millis(step.p95),
                samples = step.samples,
            ));
        }

        lines.push(format!(
            "  {label:<21} median {median:>9}  p95 {p95:>9}  (nearest-rank p95)",
            label = "TOTAL",
            median = millis(self.total_median),
            p95 = millis(self.total_p95),
        ));

        lines.extend(self.anomaly_lines());
        lines
    }

    fn anomaly_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        if self.discarded_runs > 0 {
            lines.push(format!(
                "  note: {} run(s) discarded (never finished, or empty)",
                self.discarded_runs
            ));
        }

        if self.ignored_marks > 0 {
            lines.push(format!(
                "  note: {} mark(s) ignored (no open run, duplicate, or cap reached)",
                self.ignored_marks
            ));
        }

        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn median_of_an_odd_count_is_the_middle_value_of_the_sorted_series() {
        // Fed out of order on purpose: a median that forgets to sort would
        // return 30 here.
        let samples = [ms(30), ms(10), ms(20)];

        assert_eq!(median(&samples), Some(ms(20)));
    }

    #[test]
    fn median_of_an_even_count_averages_the_two_middle_values() {
        // Sorted: 10, 20, 30, 40. Middles are 20 and 30, mean 25. Picking the
        // lower middle - the shortcut a nearest-rank p50 would take - gives 20
        // and fails this test.
        let samples = [ms(40), ms(10), ms(30), ms(20)];

        assert_eq!(median(&samples), Some(ms(25)));
    }

    #[test]
    fn an_even_median_is_not_truncated_to_whole_milliseconds() {
        // 10 and 21 average to 15.5 ms exactly. Integer millisecond arithmetic
        // would yield 15 ms; the computation runs in nanoseconds.
        let samples = [ms(10), ms(21)];

        assert_eq!(median(&samples), Some(Duration::from_micros(15_500)));
    }

    #[test]
    fn there_is_no_median_of_an_empty_series() {
        assert_eq!(median(&[]), None, "zero would be a fabricated measurement");
    }

    #[test]
    fn p95_of_twenty_samples_is_the_nineteenth_smallest() {
        // Nearest rank, inclusive: ceil(95 * 20 / 100) = 19, so the 19th of the
        // sorted series, which is 19 ms here. Fed shuffled, so a version that
        // read the raw order would return something else.
        const SHUFFLED: [u64; 20] = [
            7, 3, 20, 11, 1, 16, 5, 19, 9, 14, 2, 18, 6, 12, 4, 17, 8, 15, 10, 13,
        ];
        let samples: Vec<Duration> = SHUFFLED.into_iter().map(ms).collect();

        assert_eq!(samples.len(), 20, "the hand-computed rank assumes n = 20");
        assert_eq!(percentile(&samples, 95), Some(ms(19)));
        assert_ne!(
            percentile(&samples, 95),
            Some(ms(20)),
            "p95 on 20 samples must not collapse onto the maximum"
        );
    }

    #[test]
    fn p95_of_ten_samples_is_the_maximum_which_is_why_twenty_runs_are_required() {
        // ceil(95 * 10 / 100) = 10. Documented consequence of nearest rank on a
        // small N, asserted so nobody later reads a ten-run p95 as a percentile.
        let samples: Vec<Duration> = (1..=10).map(ms).collect();

        assert_eq!(percentile(&samples, 10), Some(ms(1)));
        assert_eq!(percentile(&samples, 95), Some(ms(10)));
        assert_eq!(percentile(&samples, 100), Some(ms(10)));
    }

    #[test]
    fn a_percentile_rank_is_never_zero_and_never_past_the_end() {
        let samples = [ms(5), ms(1), ms(3)];

        assert_eq!(percentile(&samples, 0), Some(ms(1)), "clamped up to rank 1");
        assert_eq!(percentile(&samples, 100), Some(ms(5)));
        assert_eq!(percentile(&[], 95), None);
        assert_eq!(percentile(&[ms(42)], 95), Some(ms(42)));
    }

    #[test]
    fn a_non_monotonic_offset_yields_zero_rather_than_a_wrapped_duration() {
        // Instant is monotonic by contract, so this should never happen. If it
        // ever does, a plain subtraction panics in debug and wraps to ~584
        // years in release. Both are worse than a zero.
        let steps = [
            Step {
                label: "capture",
                at: ms(10),
            },
            Step {
                label: "paint",
                at: ms(5),
            },
        ];

        let gaps = step_gaps(&steps);

        assert_eq!(gaps, vec![("capture", ms(10)), ("paint", Duration::ZERO)]);
    }

    #[test]
    fn a_gap_is_the_time_of_the_step_itself_not_the_offset_from_the_start() {
        let steps = [
            Step {
                label: "capture",
                at: ms(10),
            },
            Step {
                label: "paint",
                at: ms(35),
            },
            Step {
                label: "clipboard",
                at: ms(40),
            },
        ];

        let gaps = step_gaps(&steps);

        assert_eq!(
            gaps,
            vec![("capture", ms(10)), ("paint", ms(25)), ("clipboard", ms(5))]
        );
    }

    #[test]
    fn steps_keep_the_order_in_which_they_were_recorded() {
        let timings = Timings::new();
        timings.begin_run();
        timings.mark("shortcut");
        timings.mark("capture");
        timings.mark("paint");
        timings.finish_run();

        let labels: Vec<&str> = timings
            .report()
            .steps
            .iter()
            .map(|step| step.label)
            .collect();

        // Not sorted, not hashed: pipeline order. Alphabetical order would be
        // capture, paint, shortcut - a different vector.
        assert_eq!(labels, vec!["shortcut", "capture", "paint"]);
    }

    #[test]
    fn statistics_are_computed_per_step_across_every_run() {
        let timings = Timings::new();
        // Offsets, as `mark` records them. Gaps by hand:
        //   capture: 10, 20, 30   -> sorted 10 20 30, median 20, p95 (rank 3) 30
        //   paint:    5, 20,  3   -> sorted  3  5 20, median  5, p95 (rank 3) 20
        //   totals:  15, 40, 33   -> sorted 15 33 40, median 33, p95 (rank 3) 40
        timings.file_run(&[("capture", ms(10)), ("paint", ms(15))]);
        timings.file_run(&[("capture", ms(20)), ("paint", ms(40))]);
        timings.file_run(&[("capture", ms(30)), ("paint", ms(33))]);

        let report = timings.report();

        assert_eq!(report.runs, 3);
        assert_eq!(
            report.steps,
            vec![
                StepStats {
                    label: "capture",
                    median: ms(20),
                    p95: ms(30),
                    samples: 3,
                },
                StepStats {
                    label: "paint",
                    median: ms(5),
                    p95: ms(20),
                    samples: 3,
                },
            ]
        );
        assert_eq!(report.total_median, ms(33));
        assert_eq!(report.total_p95, ms(40));
    }

    #[test]
    fn a_step_missing_from_some_runs_reports_its_own_sample_count() {
        let timings = Timings::new();
        timings.file_run(&[("capture", ms(10)), ("clipboard", ms(30))]);
        timings.file_run(&[("capture", ms(20))]);

        let report = timings.report();

        assert_eq!(report.runs, 2);
        assert_eq!(report.steps[0].samples, 2);
        assert_eq!(report.steps[1].label, "clipboard");
        assert_eq!(
            report.steps[1].samples, 1,
            "a conditional step must not silently borrow the run count"
        );
    }

    #[test]
    fn a_report_without_any_run_says_so_instead_of_panicking() {
        let report = Timings::new().report();

        assert_eq!(report.runs, 0);
        assert!(report.steps.is_empty());
        assert_eq!(report.total_median, Duration::ZERO);
        assert_eq!(
            report.lines(),
            vec!["timing: no finished run to report on".to_owned()]
        );
    }

    #[test]
    fn an_unfinished_run_is_absent_from_the_report() {
        let timings = Timings::new();
        timings.begin_run();
        timings.mark("shortcut");

        let report = timings.report();

        assert_eq!(
            report.runs, 0,
            "a run counts once it is finished, not before"
        );
        assert!(report.steps.is_empty());
    }

    #[test]
    fn recording_the_same_step_twice_keeps_the_first_and_counts_the_second() {
        let timings = Timings::new();
        timings.begin_run();
        timings.mark("paint");
        timings.mark("paint");
        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.runs, 1);
        assert_eq!(report.steps.len(), 1, "one label, one line");
        assert_eq!(report.ignored_marks, 1, "the duplicate has to be visible");
    }

    #[test]
    fn marking_outside_of_a_run_is_counted_not_panicked_on() {
        let timings = Timings::new();
        timings.mark("paint");
        timings.finish_run();
        timings.abandon_run();

        let report = timings.report();

        assert_eq!(report.runs, 0);
        assert_eq!(report.ignored_marks, 1);
        assert_eq!(
            report.discarded_runs, 0,
            "finishing or abandoning nothing is a no-op, not a discarded run"
        );
    }

    #[test]
    fn restarting_a_run_discards_the_open_one() {
        let timings = Timings::new();
        timings.begin_run();
        timings.mark("shortcut");
        timings.begin_run();
        timings.mark("shortcut");
        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.runs, 1, "the abandoned half run must not be filed");
        assert_eq!(report.discarded_runs, 1);
    }

    #[test]
    fn a_finished_run_with_no_step_is_discarded_rather_than_counted_as_zero() {
        let timings = Timings::new();
        timings.begin_run();
        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.runs, 0);
        assert_eq!(report.discarded_runs, 1);
        assert_eq!(
            report.total_median,
            Duration::ZERO,
            "an empty run must not contribute a zero total to the aggregate"
        );
    }

    #[test]
    fn an_abandoned_run_is_not_reported_as_a_measurement() {
        let timings = Timings::new();
        timings.begin_run();
        timings.mark("shortcut");
        timings.abandon_run();

        let report = timings.report();

        assert_eq!(report.runs, 0, "Escape cancels, it does not measure");
        assert_eq!(report.discarded_runs, 1);
    }

    #[test]
    fn the_history_stays_bounded() {
        let timings = Timings::new();
        for _ in 0..(MAX_RUNS + 25) {
            timings.begin_run();
            timings.mark("capture");
            timings.finish_run();
        }

        assert_eq!(timings.report().runs, MAX_RUNS);
    }

    #[test]
    fn a_run_cannot_record_more_steps_than_the_cap() {
        let timings = Timings::new();
        timings.begin_run();
        // Distinct labels, so nothing is rejected as a duplicate: the cap is
        // what has to stop this.
        for label in LABELS {
            timings.mark(label);
        }
        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.steps.len(), MAX_STEPS_PER_RUN);
        assert_eq!(report.ignored_marks, LABELS.len() - MAX_STEPS_PER_RUN);
    }

    /// 70 distinct labels, above `MAX_STEPS_PER_RUN` (64), for the cap test.
    /// `&'static str` cannot be generated at run time, hence the literal list.
    const LABELS: [&str; 70] = [
        "s00", "s01", "s02", "s03", "s04", "s05", "s06", "s07", "s08", "s09", "s10", "s11", "s12",
        "s13", "s14", "s15", "s16", "s17", "s18", "s19", "s20", "s21", "s22", "s23", "s24", "s25",
        "s26", "s27", "s28", "s29", "s30", "s31", "s32", "s33", "s34", "s35", "s36", "s37", "s38",
        "s39", "s40", "s41", "s42", "s43", "s44", "s45", "s46", "s47", "s48", "s49", "s50", "s51",
        "s52", "s53", "s54", "s55", "s56", "s57", "s58", "s59", "s60", "s61", "s62", "s63", "s64",
        "s65", "s66", "s67", "s68", "s69",
    ];

    #[test]
    fn the_report_lines_carry_the_figures_and_the_anomalies() {
        let timings = Timings::new();
        timings.file_run(&[("capture", ms(12)), ("paint", ms(30))]);
        timings.mark("stray");

        let lines = timings.report().lines();

        assert_eq!(lines[0], "timing report over 1 run(s)");
        assert!(
            lines[1].starts_with("  #1 capture") && lines[1].contains("median   12.0 ms"),
            "unexpected step line: {}",
            lines[1]
        );
        assert!(
            lines[3].contains("TOTAL") && lines[3].contains("median   30.0 ms"),
            "unexpected total line: {}",
            lines[3]
        );
        assert!(
            lines[4].contains("1 mark(s) ignored"),
            "an ignored mark must reach the printed report, not just the struct: {}",
            lines[4]
        );
    }

    #[test]
    fn measured_steps_come_out_in_a_plausible_order_on_a_real_clock() {
        // The only test that uses the real clock, and it asserts an ordering,
        // never a value: a sleep of 5 ms lasts *at least* 5 ms, so an upper
        // bound here would be a coin toss on a loaded machine.
        let timings = Timings::new();
        timings.begin_run();
        std::thread::sleep(ms(5));
        timings.mark("capture");
        std::thread::sleep(ms(5));
        timings.mark("paint");
        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.runs, 1);
        assert!(
            report.steps[0].median >= ms(5),
            "a 5 ms sleep cannot be measured as {:?}",
            report.steps[0].median
        );
        assert!(
            report.total_median >= ms(10),
            "the total must cover both sleeps, got {:?}",
            report.total_median
        );
        assert!(
            report.total_median >= report.steps[0].median + report.steps[1].median,
            "the total cannot be shorter than the sum of its parts"
        );
    }

    #[test]
    fn the_instrument_is_usable_from_several_threads() {
        // What `app.manage` will do: one shared reference, marks arriving from
        // threads that never met. This is a compile-time proof as much as a
        // run-time one - it does not build unless `Timings` is `Sync` - and it
        // checks that concurrent marking neither panics nor deadlocks.
        let timings = Timings::new();
        timings.begin_run();

        std::thread::scope(|scope| {
            for label in ["capture", "paint", "clipboard"] {
                let instrument = &timings;
                scope.spawn(move || instrument.mark(label));
            }
        });

        timings.finish_run();

        let report = timings.report();

        assert_eq!(report.runs, 1);
        assert_eq!(
            report.steps.len(),
            3,
            "every thread's mark must survive, in whatever order they arrived"
        );
        assert_eq!(report.ignored_marks, 0);
    }

    #[test]
    fn a_poisoned_lock_does_not_take_the_instrument_down() {
        // A panic elsewhere in the application must not turn every later
        // `mark` into a second panic inside the shortcut handler.
        // The deliberate panic prints to stderr while the suite runs. That
        // output is expected, not a failure.
        let timings = Timings::new();
        let panicked = std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let _guard = timings.state();
                    // Semicolon on purpose: it pins the closure's return type
                    // to `()` instead of leaving `!` to inference fallback.
                    panic!("poisoning the lock on purpose");
                })
                .join()
        });

        assert!(panicked.is_err(), "the helper thread was meant to panic");
        assert!(timings.inner.is_poisoned(), "the lock must now be poisoned");

        timings.begin_run();
        timings.mark("capture");
        timings.finish_run();

        assert_eq!(
            timings.report().runs,
            1,
            "the instrument must keep working through a poisoned lock"
        );
    }

    #[test]
    fn the_instrument_costs_far_less_than_what_it_measures() {
        // Timing-dependent on purpose, with a ceiling wide enough that only a
        // real design mistake trips it: an instrument costing 10 ms to measure
        // 150 ms would falsify its own verdict. 2000 runs of 5 marks is 10 000
        // marks; 500 ms allows 50 us per mark, i.e. 250 us for a whole
        // five-step run against a 150 ms budget. This proves a CEILING, not the
        // real cost - and it runs in a debug build, where the real cost is
        // several times what the release binary will pay.
        const RUNS: usize = 2_000;
        let timings = Timings::new();

        let started = Instant::now();
        for _ in 0..RUNS {
            timings.begin_run();
            for label in ["shortcut", "capture", "encode", "paint", "clipboard"] {
                timings.mark(label);
            }
            timings.finish_run();
        }
        let elapsed = started.elapsed();

        assert!(
            elapsed < ms(500),
            "{RUNS} runs of 5 marks took {elapsed:?}; the instrument is no longer negligible"
        );
    }
}
