// ─── Background Poll Wrapper ──────────────────────────────────
//
// Every long-lived background task in this binary used to be its own
// hand-rolled `tokio::spawn` + timer + `select!` + `watch` shutdown
// loop. The nine copies differed only in cadence and body, so the
// shutdown wiring — the part that actually has to be correct — was
// written nine times and had to be re-audited nine times.
//
// This module owns that wiring once. `spawn_poll` keeps the shutdown
// contract byte-for-byte: `shutdown.changed()` races the timer and the
// loop breaks only when the current value is `true`.

use std::future::Future;
use std::time::Duration;

use tokio::sync::watch;

/// When the first body run happens, relative to task start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstTick {
    /// Run at startup, then every `period`.
    ///
    /// This is what `tokio::time::interval(period)` does: its first
    /// `tick()` is ready immediately, so a task that fell behind while
    /// the process was down catches up once at boot.
    Now,
    /// Wait one full `period` before the first run.
    ///
    /// This is what a `tokio::time::sleep(period)` created inside the
    /// loop body does: the clock starts when the loop starts, so
    /// startup does not fire an extra DB round-trip. Kept distinct
    /// from `Now` because collapsing the two would add a query at boot.
    AfterDelay,
}

/// Spawn a detached background task that runs `task` on a fixed cadence
/// until `shutdown` resolves to `true`.
///
/// `task` is a closure producing a fresh future per run, so callers move
/// the pool / registry / broadcaster into it. It is `FnMut` (not `Fn`) so
/// a caller whose body has a genuine first-run-only prologue — e.g. the
/// scheduler's boot-time reclaim — can gate that with a captured `bool`
/// instead of spawning a second task for it.
///
/// `shutdown_log` is emitted once, verbatim, on the way out. Every loop
/// this replaces carried its own "… shutting down" line, and an operator
/// reading the logs at restart should not lose them.
///
/// Timing matches the loop it replaces: `FirstTick::Now` uses one
/// `tokio::time::interval` with `MissedTickBehavior::Skip`, exactly as
/// before; `FirstTick::AfterDelay` awaits a fresh `sleep(period)` each
/// iteration, so a body that overruns its period still waits a full
/// period before the next run.
pub fn spawn_poll<F, Fut>(
    period: Duration,
    first: FirstTick,
    mut shutdown: watch::Receiver<bool>,
    shutdown_log: &'static str,
    mut task: F,
) where
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send,
{
    tokio::spawn(async move {
        let mut ticker = match first {
            FirstTick::Now => Some(tokio::time::interval(period)),
            FirstTick::AfterDelay => None,
        };

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        tracing::info!("{shutdown_log}");
                        break;
                    }
                }
                _ = next_tick(&mut ticker, period) => (task)().await,
            }
        }
    });
}

/// Await the next scheduled wake-up, honouring the caller's cadence.
///
/// `Some` is the shared `interval` (ready immediately, then every
/// `period`); `None` is the per-iteration `sleep` (always a full period
/// after the previous run, never immediately after an overrun).
async fn next_tick(ticker: &mut Option<tokio::time::Interval>, period: Duration) {
    match ticker {
        Some(t) => {
            t.tick().await;
        }
        None => tokio::time::sleep(period).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A poll task must stop as soon as the watch channel reads `true`,
    /// even when it is waiting on a timer that is nowhere near firing.
    #[tokio::test]
    async fn spawn_poll_stops_when_shutdown_flips_true() {
        let (tx, rx) = watch::channel(false);
        let runs = Arc::new(AtomicUsize::new(0));

        spawn_poll(
            Duration::from_secs(3600),
            FirstTick::AfterDelay,
            rx,
            "shutting down",
            {
                let runs = runs.clone();
                move || {
                    runs.fetch_add(1, Ordering::SeqCst);
                    async {}
                }
            },
        );

        // Nothing should have run yet: this cadence never fires in-test.
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(runs.load(Ordering::SeqCst), 0, "body ran before its period");

        tx.send(true).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(runs.load(Ordering::SeqCst), 0, "body ran after shutdown");
    }

    /// The shutdown arm must win over a timer that is already ready, and
    /// the loop must then stay stopped. `select!` picks randomly among
    /// ready branches, so the body legitimately runs a few more times
    /// after `send(true)` — what must not happen is for it to keep
    /// running. Regression guard: wiring the timer arm so it always wins
    /// spins the body forever.
    #[tokio::test]
    async fn spawn_poll_stays_stopped_after_shutdown_even_with_a_ready_timer() {
        let (tx, rx) = watch::channel(false);
        let runs = Arc::new(AtomicUsize::new(0));

        spawn_poll(
            Duration::from_millis(1),
            FirstTick::Now,
            rx,
            "shutting down",
            {
                let runs = runs.clone();
                move || {
                    runs.fetch_add(1, Ordering::SeqCst);
                    async {}
                }
            },
        );

        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            runs.load(Ordering::SeqCst) > 0,
            "FirstTick::Now body never ran at startup"
        );

        tx.send(true).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        let settled = runs.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert_eq!(
            runs.load(Ordering::SeqCst),
            settled,
            "body kept running after shutdown"
        );
    }

    /// `FirstTick::Now` runs at startup; `FirstTick::AfterDelay` does
    /// not. This is the difference that keeps a boot-time DB query out
    /// of the delay-shaped tasks.
    #[tokio::test]
    async fn first_tick_controls_whether_the_body_runs_at_startup() {
        for (first, expected_at_startup) in [(FirstTick::Now, true), (FirstTick::AfterDelay, false)] {
            let (tx, rx) = watch::channel(false);
            let runs = Arc::new(AtomicUsize::new(0));

            spawn_poll(
                Duration::from_secs(3600),
                first,
                rx,
                "shutting down",
                {
                    let runs = runs.clone();
                    move || {
                        runs.fetch_add(1, Ordering::SeqCst);
                        async {}
                    }
                },
            );

            tokio::time::sleep(Duration::from_millis(30)).await;
            assert_eq!(
                runs.load(Ordering::SeqCst),
                usize::from(expected_at_startup),
                "unexpected startup behaviour for {first:?}"
            );

            tx.send(true).unwrap();
        }
    }

    /// A `false` notification must not stop the loop — only a `true`
    /// does. Mirrors the `if *shutdown.borrow()` guard the original
    /// loops each carried.
    #[tokio::test]
    async fn a_false_notification_does_not_stop_the_poll() {
        let (tx, rx) = watch::channel(false);
        let runs = Arc::new(AtomicUsize::new(0));

        spawn_poll(
            Duration::from_millis(1),
            FirstTick::Now,
            rx,
            "shutting down",
            {
                let runs = runs.clone();
                move || {
                    runs.fetch_add(1, Ordering::SeqCst);
                    async {}
                }
            },
        );

        tokio::time::sleep(Duration::from_millis(20)).await;
        let before = runs.load(Ordering::SeqCst);

        tx.send(false).unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            runs.load(Ordering::SeqCst) > before,
            "loop stopped on a false shutdown notification"
        );

        tx.send(true).unwrap();
    }
}