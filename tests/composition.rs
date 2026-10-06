use cmmr::conformance::{TestClock, limits, run, write};
use cvtl::*;
use std::{
    num::NonZeroUsize,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
// Only clock and transport loss are controlled. The production scripts execute
// in the actual disposable primary; no replacement store logic exists here.
struct Controlled {
    connection: redis::aio::MultiplexedConnection,
    real: RedisNetwork,
    clock: TestClock,
    lose_reply: AtomicBool,
}
impl Network for Controlled {
    async fn health(&self) -> Result<cvlk::Health, Error> {
        self.real.health().await
    }
    async fn invoke(
        &self,
        script: &'static str,
        key: &str,
        request: &str,
    ) -> Result<String, Error> {
        let now = self.clock.now()?;
        let script = script.replace(
            "local t = redis.call('TIME')",
            &format!("local t = {{'{}','{}'}}", now / 1000, (now % 1000) * 1000),
        );
        let result = redis::Script::new(&script)
            .key(key)
            .arg(request)
            .invoke_async(&mut self.connection.clone())
            .await
            .map_err(|_| Error::Unavailable)?;
        if self.lose_reply.load(Ordering::SeqCst) {
            Err(Error::Unavailable)
        } else {
            Ok(result)
        }
    }
}
// Same refusal vector through both real facade variants: closing one scope
// through a clone refuses on all its clones while an independent scope on the
// same backend keeps its record and stays writable.
async fn refusal_isolation<S>(victim: &S, survivor: &S, live: u64, replacement: u64)
where
    S: Store + Clone,
{
    victim
        .compare_exchange(&[write("refusal", "g", 1, live)])
        .await
        .unwrap();
    survivor
        .compare_exchange(&[write("refusal", "g", 1, live)])
        .await
        .unwrap();
    assert_eq!(
        victim.get("refusal").await.unwrap().unwrap().revision,
        Revision([1; 16])
    );
    assert_eq!(
        survivor.get("refusal").await.unwrap().unwrap().revision,
        Revision([1; 16])
    );
    let through = victim.clone();
    through.close();
    assert_eq!(victim.state(), State::Closed);
    assert_eq!(through.state(), State::Closed);
    assert_eq!(victim.get("refusal").await, Err(Error::Closed));
    assert_eq!(through.get("refusal").await, Err(Error::Closed));
    assert_eq!(
        victim
            .compare_exchange(&[write("refusal", "g", 9, live)])
            .await,
        Err(Error::Closed)
    );
    assert_eq!(
        through
            .compare_exchange(&[write("refusal", "g", 9, live)])
            .await,
        Err(Error::Closed)
    );
    assert_eq!(victim.maintain().await, Err(Error::Closed));
    assert_eq!(through.maintain().await, Err(Error::Closed));
    assert_eq!(victim.state(), State::Closed);
    assert_eq!(through.state(), State::Closed);
    assert_eq!(survivor.state(), State::Ready);
    assert_eq!(
        survivor.get("refusal").await.unwrap().unwrap().revision,
        Revision([1; 16])
    );
    let mut update = write("refusal", "g", 2, replacement);
    update.expected = Some(Revision([1; 16]));
    survivor.compare_exchange(&[update]).await.unwrap();
    assert_eq!(
        survivor.get("refusal").await.unwrap().unwrap().revision,
        Revision([2; 16])
    );
    assert_eq!(survivor.state(), State::Ready);
}
#[tokio::test]
async fn identical_suite_through_both_facade_variants() {
    let start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
        + 120_000;
    let clock = TestClock::new(start);
    let a: Volatile<_> = Volatile::memory(
        Scope::new("memory-a", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .unwrap();
    let b: Volatile<_> = Volatile::memory(
        Scope::new("memory-b", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .unwrap();
    assert!(a.warnings().is_empty());
    assert!(a.reopen().await.unwrap().is_empty());
    assert_eq!(a.clone().state(), State::Ready);
    assert_eq!(a.maintain().await.unwrap(), 0);
    let victim: Volatile<_> = Volatile::memory(
        Scope::new("memory-victim", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .unwrap();
    let survivor: Volatile<_> = Volatile::memory(
        Scope::new("memory-survivor", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .unwrap();
    refusal_isolation(&victim, &survivor, start + 5000, start + 6000).await;
    assert!(victim.reopen().await.unwrap().is_empty());
    assert_eq!(victim.state(), State::Closed);
    assert_eq!(survivor.state(), State::Ready);
    run(&a, &b, &clock).await;
    assert_eq!(format!("{a:?}"), "Volatile(Memory)");
    assert!(a.reopen().await.unwrap().is_empty());
    assert_eq!(a.state(), State::Closed);
    let bad: Result<Volatile<_>, _> = Volatile::memory(
        Scope::new("bad", "facade").unwrap(),
        Limits {
            records: 0,
            ..limits()
        },
        clock.clone(),
    );
    assert!(matches!(bad, Err(Error::Invalid)));
    clock.set(start);
    let connection =
        redis::Client::open(std::env::var("VALKEY_URL").expect("disposable Valkey required"))
            .unwrap()
            .get_multiplexed_async_connection()
            .await
            .unwrap();
    let network = Arc::new(Controlled {
        real: RedisNetwork::new(connection.clone(), Duration::from_secs(5)).unwrap(),
        connection,
        clock: clock.clone(),
        lose_reply: AtomicBool::new(false),
    });
    let a = Volatile::valkey(
        network.clone(),
        Scope::new("valkey-a", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .await
    .unwrap();
    let b = Volatile::valkey(
        network.clone(),
        Scope::new("valkey-b", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .await
    .unwrap();
    assert_eq!(a.warnings(), network.health().await.unwrap().warnings);
    assert_eq!(
        a.reopen().await.unwrap(),
        network.health().await.unwrap().warnings
    );
    assert_eq!(a.clone().state(), State::Ready);
    assert_eq!(a.maintain().await.unwrap(), 0);
    let victim = Volatile::valkey(
        network.clone(),
        Scope::new("valkey-victim", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .await
    .unwrap();
    let survivor = Volatile::valkey(
        network.clone(),
        Scope::new("valkey-survivor", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .await
    .unwrap();
    refusal_isolation(&victim, &survivor, start + 5000, start + 6000).await;
    assert_eq!(victim.reopen().await, Err(Error::Closed));
    assert_eq!(victim.state(), State::Closed);
    assert_eq!(survivor.state(), State::Ready);
    run(&a, &b, &clock).await;
    assert_eq!(format!("{a:?}"), "Volatile(Valkey)");
    assert_eq!(a.reopen().await, Err(Error::Closed));
    clock.set(start);
    let quota_store = Volatile::valkey(
        network.clone(),
        Scope::new("quota", "facade").unwrap(),
        limits(),
        clock.clone(),
    )
    .await
    .unwrap();
    let Volatile::Valkey(child) = &quota_store else {
        unreachable!()
    };
    let policy = cthl::Limit::new(2, Duration::from_millis(50)).unwrap();
    let distributed = cthl::Throttle::new(
        "quota",
        [("send", policy)],
        ThrottleStore::new(child.clone(), NonZeroUsize::new(1).unwrap()).unwrap(),
    )
    .unwrap();
    let relative = cthl::FakeRelativeClock::default();
    let memory = cthl::Throttle::new(
        "quota",
        [("send", policy)],
        cthl::MemoryStore::with_clock(NonZeroUsize::new(1).unwrap(), relative.clone()),
    )
    .unwrap();
    let mut last = 0;
    for offset in [0, 0, 0, 49, 50, 99, 100] {
        clock.set(start + offset);
        relative.advance(Duration::from_millis(offset - last));
        last = offset;
        assert_eq!(
            distributed.check(b"opaque", "send").await,
            memory.check(b"opaque", "send").await
        );
    }
    assert_eq!(
        distributed.check(b"capacity", "send").await,
        Err(cthl::Error::Store(cthl::StoreError::CapacityExhausted))
    );
    network.lose_reply.store(true, Ordering::SeqCst);
    assert_eq!(
        quota_store
            .compare_exchange(&[write("entry", "g", 1, start + 5000)])
            .await,
        Err(Error::Unavailable)
    );
    assert_eq!(quota_store.state(), State::Unavailable);
    assert_eq!(quota_store.get("entry").await, Err(Error::Unavailable));
    network.lose_reply.store(false, Ordering::SeqCst);
    quota_store.reopen().await.unwrap();
    assert_eq!(
        quota_store.get("entry").await.unwrap().unwrap().revision,
        Revision([1; 16])
    );
    quota_store.close();
    assert_eq!(
        distributed.check(b"opaque", "send").await,
        Err(cthl::Error::Store(cthl::StoreError::Unavailable))
    );
    assert!(matches!(
        Volatile::valkey(
            network,
            Scope::new("bad", "facade").unwrap(),
            Limits {
                records: 0,
                ..limits()
            },
            clock,
        )
        .await,
        Err(Error::Invalid)
    ));
    // Closing one cloned facade closes its child; no independent facade state.
    assert_eq!(a.clone().state(), State::Closed);
}
