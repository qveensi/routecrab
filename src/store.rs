use std::sync::{Arc, RwLock};

use indexmap::IndexMap;
use tokio::sync::broadcast;

use crate::model::{HealthStatus, Route};

#[derive(Clone, Debug)]
pub enum Change {
    Upsert(Box<Route>),
    Remove(String),
}

#[derive(Clone)]
pub struct Store {
    // Lock access recovers from poisoning (`unwrap_or_else(into_inner)`) instead
    // of panicking: the guarded data is plain owned structs, so a panic in one
    // request must not cascade and take the whole dashboard down.
    inner: Arc<RwLock<IndexMap<String, Route>>>,
    tx: broadcast::Sender<Change>,
}

impl Store {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            inner: Arc::new(RwLock::new(IndexMap::new())),
            tx,
        }
    }

    pub fn upsert(&self, r: Route) {
        self.inner
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .insert(r.id.clone(), r.clone());
        // ignore error: no active subscribers is expected when idle
        let _ = self.tx.send(Change::Upsert(Box::new(r)));
    }

    pub fn remove(&self, id: &str) {
        self.inner
            .write()
            .unwrap_or_else(|e| e.into_inner())
            .shift_remove(id);
        let _ = self.tx.send(Change::Remove(id.to_owned()));
    }

    /// Returns all routes sorted by (group, order, name).
    pub fn list(&self) -> Vec<Route> {
        let mut routes: Vec<Route> = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .cloned()
            .collect();
        routes.sort_by(|a, b| {
            a.group
                .cmp(&b.group)
                .then(a.order.cmp(&b.order))
                .then(a.name.cmp(&b.name))
        });
        routes
    }

    /// Updates the health field of an existing route in place and emits `Change::Upsert`.
    /// No-op if the id is absent.
    pub fn set_health(&self, id: &str, h: HealthStatus) {
        let updated = {
            let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
            guard.get_mut(id).map(|r| {
                r.health = h;
                r.clone()
            })
        };
        if let Some(r) = updated {
            let _ = self.tx.send(Change::Upsert(Box::new(r)));
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Change> {
        self.tx.subscribe()
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Route;

    #[tokio::test]
    async fn upsert_emits_change_and_lists() {
        let s = Store::new();
        let mut rx = s.subscribe();
        s.upsert(Route {
            id: "a".into(),
            ..Default::default()
        });
        assert!(matches!(rx.recv().await.unwrap(), Change::Upsert(_)));
        assert_eq!(s.list().len(), 1);
    }

    #[tokio::test]
    async fn remove_emits_change() {
        let s = Store::new();
        let mut rx = s.subscribe();
        s.upsert(Route {
            id: "b".into(),
            ..Default::default()
        });
        // consume upsert event
        let _ = rx.recv().await.unwrap();
        s.remove("b");
        assert!(matches!(rx.recv().await.unwrap(), Change::Remove(_)));
        assert_eq!(s.list().len(), 0);
    }

    #[tokio::test]
    async fn set_health_updates_in_place_and_emits() {
        let s = Store::new();
        let mut rx = s.subscribe();
        s.upsert(Route {
            id: "c".into(),
            ..Default::default()
        });
        let _ = rx.recv().await.unwrap();
        s.set_health("c", HealthStatus::Healthy);
        match rx.recv().await.unwrap() {
            Change::Upsert(r) => assert_eq!(r.health, HealthStatus::Healthy),
            _ => panic!("expected Upsert"),
        }
        assert_eq!(s.list()[0].health, HealthStatus::Healthy);
    }

    #[test]
    fn list_sorted_by_group_order_name() {
        let s = Store::new();
        s.upsert(Route {
            id: "1".into(),
            group: "b".into(),
            order: 1,
            name: "z".into(),
            ..Default::default()
        });
        s.upsert(Route {
            id: "2".into(),
            group: "a".into(),
            order: 2,
            name: "m".into(),
            ..Default::default()
        });
        s.upsert(Route {
            id: "3".into(),
            group: "a".into(),
            order: 1,
            name: "n".into(),
            ..Default::default()
        });
        s.upsert(Route {
            id: "4".into(),
            group: "a".into(),
            order: 1,
            name: "a".into(),
            ..Default::default()
        });
        let list = s.list();
        assert_eq!(list[0].id, "4"); // group=a, order=1, name=a
        assert_eq!(list[1].id, "3"); // group=a, order=1, name=n
        assert_eq!(list[2].id, "2"); // group=a, order=2
        assert_eq!(list[3].id, "1"); // group=b
    }
}
