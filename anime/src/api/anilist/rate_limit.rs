//! Global rate limiting controls for AniList.
//!
//! The effective rate limit can be accessed / changed with the [`REFILL_PERMIT_EVERY`] mutex, and
//! the initial / continuous burst amount can be controlled with the [`BURST_AMOUNT`] mutex. Care
//! should be taken to ensure neither mutex is held any longer than necessary if modifying them,
//! to avoid holding up any requests when there are no more permits left.
//!
//! ## "Why are global mutexes used?"
//!
//! The AniList API does not require authentication, so scoping rate limits to an individual AniList API instance
//! could lead to rate limits being hit if multiple API instances are used throughout an application (ex. to manage
//! multiple accounts).

use parking_lot::Mutex;
use std::{
    sync::{Arc, LazyLock},
    time::Duration,
};
use tokio::sync::{Notify, Semaphore};

/// How often to add a request permit when any have been used.
/// Defaults to 2 seconds (30 requests per minute).
///
/// AniList documents its current limit here:
/// https://docs.anilist.co/guide/rate-limiting#rate-limiting
pub static REFILL_PERMIT_EVERY: Mutex<Duration> = Mutex::new(Duration::from_secs(2));

/// The maximum number of requests that can be sent in a burst, without any delays.
/// Defaults to 10.
pub static BURST_AMOUNT: Mutex<usize> = Mutex::new(10);

static PERMIT_ACQUIRED: LazyLock<Arc<Notify>> = LazyLock::new(|| Arc::new(Notify::new()));

static RATE_LIMIT_PERMITS: LazyLock<Arc<Semaphore>> = LazyLock::new(|| {
    let semaphore = Arc::new(Semaphore::new(*BURST_AMOUNT.lock()));

    let sem_clone = semaphore.clone();
    let notif_clone = PERMIT_ACQUIRED.clone();

    tokio::spawn(async move { refill_permits(sem_clone, notif_clone).await });

    semaphore
});

async fn refill_permits(semaphore: Arc<Semaphore>, permit_acquired: Arc<Notify>) {
    loop {
        // access mutexes in their own block to ensure the lock is held as
        // little as possible
        let sleep_dur = { *REFILL_PERMIT_EVERY.lock() };
        tokio::time::sleep(sleep_dur).await;

        if semaphore.available_permits() < { *BURST_AMOUNT.lock() } {
            tracing::trace!("adding rate limit permit");
            semaphore.add_permits(1);
        } else {
            tracing::trace!("max rate limit permits acquired");
            permit_acquired.notified().await;
        }
    }
}

/// Acquire a permit to execute a request. This will wait until a permit is available if necessary.
pub async fn acquire_permit() {
    tracing::trace!("acquiring rate limit permit");

    PERMIT_ACQUIRED.notify_waiters();

    let permit = RATE_LIMIT_PERMITS
        .acquire()
        .await
        // safety: the semaphore is never closed
        .expect("rate limit semaphore was closed");

    tracing::trace!("rate limit permit acquired");

    // prevent the semaphore from refilling this permit
    permit.forget();
}
