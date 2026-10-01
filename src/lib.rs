//! Select one real volatile backend at startup; all state and semantics remain
//! in the chosen child. There is no fallback, domain policy or duplicate ledger.
#![forbid(unsafe_code)]
pub use cmmr::{
    Clock, Cursor, Deadline, Error, Limits, Page, Record, Revision, Scope, State, Store, Write,
};
pub use cvlk::{Network, RedisNetwork, StartupError, ThrottleStore, Warning, cthl};
use std::sync::Arc;

pub enum Volatile<C, N = RedisNetwork> {
    Memory(Arc<cmmr::Memory<C>>),
    Valkey(Arc<cvlk::Valkey<N, C>>),
}
impl<C, N> std::fmt::Debug for Volatile<C, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Memory(_) => "Volatile(Memory)",
            Self::Valkey(_) => "Volatile(Valkey)",
        })
    }
}
impl<C, N> Clone for Volatile<C, N> {
    fn clone(&self) -> Self {
        match self {
            Self::Memory(s) => Self::Memory(s.clone()),
            Self::Valkey(s) => Self::Valkey(s.clone()),
        }
    }
}
impl<C: Clock, N: Network> Volatile<C, N> {
    pub fn memory(scope: Scope, limits: Limits, clock: C) -> Result<Self, Error> {
        Ok(Self::Memory(Arc::new(cmmr::Memory::new(
            scope, limits, clock,
        )?)))
    }
    pub async fn valkey(
        network: Arc<N>,
        scope: Scope,
        limits: Limits,
        clock: C,
    ) -> Result<Self, Error> {
        Ok(Self::Valkey(Arc::new(
            cvlk::Valkey::open(network, scope, limits, clock).await?,
        )))
    }
    /// Recheck the selected backend after an uncertain mutation or during the
    /// service maintenance probe. Memory is a no-op; it never revives a closed
    /// child. Valkey requires the same backend run ID and never retries writes.
    pub async fn reopen(&self) -> Result<Vec<Warning>, Error> {
        match self {
            Self::Memory(_) => Ok(Vec::new()),
            Self::Valkey(s) => s.reopen().await,
        }
    }
    pub fn warnings(&self) -> &[Warning] {
        match self {
            Self::Memory(_) => &[],
            Self::Valkey(s) => s.warnings(),
        }
    }
}
impl<C: Clock, N: Network> Store for Volatile<C, N> {
    fn state(&self) -> State {
        match self {
            Self::Memory(s) => s.state(),
            Self::Valkey(s) => s.state(),
        }
    }
    async fn get(&self, key: &str) -> Result<Option<Record>, Error> {
        match self {
            Self::Memory(s) => s.get(key).await,
            Self::Valkey(s) => s.get(key).await,
        }
    }
    async fn compare_exchange(&self, batch: &[Write]) -> Result<(), Error> {
        match self {
            Self::Memory(s) => s.compare_exchange(batch).await,
            Self::Valkey(s) => s.compare_exchange(batch).await,
        }
    }
    async fn delete_generation(&self, generation: &str) -> Result<usize, Error> {
        match self {
            Self::Memory(s) => s.delete_generation(generation).await,
            Self::Valkey(s) => s.delete_generation(generation).await,
        }
    }
    async fn page(
        &self,
        index: &str,
        cursor: Option<&Cursor>,
        limit: usize,
    ) -> Result<Page, Error> {
        match self {
            Self::Memory(s) => s.page(index, cursor, limit).await,
            Self::Valkey(s) => s.page(index, cursor, limit).await,
        }
    }
    async fn maintain(&self) -> Result<usize, Error> {
        match self {
            Self::Memory(s) => s.maintain().await,
            Self::Valkey(s) => s.maintain().await,
        }
    }
    fn close(&self) {
        match self {
            Self::Memory(s) => s.close(),
            Self::Valkey(s) => s.close(),
        }
    }
}
