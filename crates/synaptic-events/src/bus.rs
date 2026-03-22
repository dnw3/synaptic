//! EventBus — routes events to registered subscribers based on dispatch mode.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use tracing::warn;

use crate::{
    DispatchMode, EmitResult, Event, EventAction, EventFilter, EventKind, EventSubscriber,
};

// ---------------------------------------------------------------------------
// SubscriberEntry
// ---------------------------------------------------------------------------

struct SubscriberEntry {
    subscriber: Arc<dyn EventSubscriber>,
    priority: i32,
    /// Tag for diagnostics.
    tag: String,
}

// ---------------------------------------------------------------------------
// EventBus
// ---------------------------------------------------------------------------

/// Central dispatcher for all Synaptic events.
///
/// Uses `std::sync::RwLock` for interior mutability so that [`subscribe`]
/// takes `&self` and the bus can be shared via `Arc<EventBus>` without
/// requiring `&mut` access.
pub struct EventBus {
    /// Per-kind subscriber lists (for `Exact` and `AnyOf` filters).
    subscribers: RwLock<HashMap<EventKind, Vec<SubscriberEntry>>>,
    /// Subscribers registered with `EventFilter::All`.
    global_subscribers: RwLock<Vec<SubscriberEntry>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Creates an empty bus.
    pub fn new() -> Self {
        Self {
            subscribers: RwLock::new(HashMap::new()),
            global_subscribers: RwLock::new(Vec::new()),
        }
    }

    /// Register `subscriber` to receive events that match its declared filters.
    ///
    /// `priority` controls execution order — lower values run first (e.g.
    /// `-100` runs before `0` which runs before `10`).  `tag` is a
    /// human-readable label used in diagnostic messages.
    pub fn subscribe(
        &self,
        subscriber: Arc<dyn EventSubscriber>,
        priority: i32,
        tag: impl Into<String>,
    ) {
        let tag = tag.into();
        let filters = subscriber.subscriptions();

        for filter in filters {
            match filter {
                EventFilter::All => {
                    let mut globals = self.global_subscribers.write().unwrap();
                    globals.push(SubscriberEntry {
                        subscriber: Arc::clone(&subscriber),
                        priority,
                        tag: tag.clone(),
                    });
                    globals.sort_by_key(|e| e.priority);
                }
                EventFilter::Exact(kind) => {
                    let mut map = self.subscribers.write().unwrap();
                    let list = map.entry(kind).or_default();
                    list.push(SubscriberEntry {
                        subscriber: Arc::clone(&subscriber),
                        priority,
                        tag: tag.clone(),
                    });
                    list.sort_by_key(|e| e.priority);
                }
                EventFilter::AnyOf(kinds) => {
                    for kind in kinds {
                        let mut map = self.subscribers.write().unwrap();
                        let list = map.entry(kind).or_default();
                        list.push(SubscriberEntry {
                            subscriber: Arc::clone(&subscriber),
                            priority,
                            tag: tag.clone(),
                        });
                        list.sort_by_key(|e| e.priority);
                    }
                }
            }
        }
    }

    /// Remove all subscribers whose tag matches the given value.
    ///
    /// Used for plugin hot-unload: removes all subscribers registered by a
    /// specific plugin (tag = `"plugin:{id}"`).
    pub fn unsubscribe_by_tag(&self, tag: &str) -> usize {
        let mut removed = 0;
        {
            let mut map = self.subscribers.write().unwrap();
            for list in map.values_mut() {
                let before = list.len();
                list.retain(|e| e.tag != tag);
                removed += before - list.len();
            }
        }
        {
            let mut globals = self.global_subscribers.write().unwrap();
            let before = globals.len();
            globals.retain(|e| e.tag != tag);
            removed += before - globals.len();
        }
        removed
    }

    /// Returns the total number of subscribers registered for a given kind,
    /// including global (`EventFilter::All`) subscribers.
    pub fn subscriber_count(&self, kind: &EventKind) -> usize {
        let map = self.subscribers.read().unwrap();
        let kind_count = map.get(kind).map_or(0, |v| v.len());
        let global_count = self.global_subscribers.read().unwrap().len();
        kind_count + global_count
    }

    /// Dispatch `event` to all matching subscribers according to its
    /// [`DispatchMode`].
    pub async fn emit(
        &self,
        event: &mut Event,
    ) -> Result<EmitResult, synaptic_core::SynapticError> {
        let mode = event.kind.dispatch_mode();

        // Collect subscriber arcs (priority-sorted) without holding the lock
        // across await points.
        let entries: Vec<(Arc<dyn EventSubscriber>, i32, String)> = {
            let map = self.subscribers.read().unwrap();
            let globals = self.global_subscribers.read().unwrap();

            // Merge kind-specific + global entries, then sort by priority.
            let mut combined: Vec<(Arc<dyn EventSubscriber>, i32, String)> = Vec::new();

            if let Some(list) = map.get(&event.kind) {
                for e in list {
                    combined.push((Arc::clone(&e.subscriber), e.priority, e.tag.clone()));
                }
            }
            for e in globals.iter() {
                combined.push((Arc::clone(&e.subscriber), e.priority, e.tag.clone()));
            }
            combined.sort_by_key(|(_, p, _)| *p);
            combined
        };

        match mode {
            DispatchMode::Parallel => {
                // Clone the event for each subscriber, fire concurrently.
                let mut event_clones: Vec<Event> = entries.iter().map(|_| event.clone()).collect();
                let futures: Vec<_> = entries
                    .iter()
                    .zip(event_clones.iter_mut())
                    .map(|((sub, _, tag), ev)| {
                        let tag = tag.clone();
                        async move {
                            if let Err(e) = sub.handle(ev).await {
                                warn!(tag = %tag, error = %e, "parallel subscriber error (ignored)");
                            }
                        }
                    })
                    .collect();
                futures::future::join_all(futures).await;
                Ok(EmitResult::Proceed)
            }

            DispatchMode::Sequential | DispatchMode::Synchronous => {
                for (sub, _, tag) in &entries {
                    match sub.handle(event).await {
                        Ok(EventAction::Continue) | Ok(EventAction::Modify) => {}
                        Ok(EventAction::Cancel) => return Ok(EmitResult::Cancelled),
                        Ok(EventAction::Intercept(val)) => return Ok(EmitResult::Intercepted(val)),
                        Ok(EventAction::Retry) => {
                            warn!(tag = %tag, "Retry action ignored in Sequential/Synchronous mode");
                        }
                        Ok(EventAction::Error(e)) => return Err(e),
                        Err(e) => return Err(e),
                    }
                }
                Ok(EmitResult::Proceed)
            }

            DispatchMode::Intercept => {
                for (sub, _, tag) in &entries {
                    match sub.handle(event).await {
                        Ok(EventAction::Continue) | Ok(EventAction::Modify) => {}
                        Ok(EventAction::Cancel) => return Ok(EmitResult::Cancelled),
                        Ok(EventAction::Intercept(val)) => return Ok(EmitResult::Intercepted(val)),
                        Ok(EventAction::Retry) => {
                            warn!(tag = %tag, "Retry action ignored in Intercept mode");
                        }
                        Ok(EventAction::Error(e)) => return Err(e),
                        Err(e) => return Err(e),
                    }
                }
                Ok(EmitResult::Proceed)
            }

            DispatchMode::ErrorPath => {
                for (sub, _, tag) in &entries {
                    let _ = tag;
                    match sub.handle(event).await {
                        Ok(EventAction::Continue) | Ok(EventAction::Modify) => {}
                        Ok(EventAction::Cancel) => return Ok(EmitResult::Cancelled),
                        Ok(EventAction::Intercept(val)) => return Ok(EmitResult::Intercepted(val)),
                        Ok(EventAction::Retry) => return Ok(EmitResult::Retry),
                        Ok(EventAction::Error(e)) => return Err(e),
                        Err(e) => return Err(e),
                    }
                }
                Ok(EmitResult::Proceed)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct CountingSubscriber {
        count: Arc<AtomicU32>,
        kind: EventKind,
    }

    #[async_trait::async_trait]
    impl EventSubscriber for CountingSubscriber {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(self.kind)]
        }
        async fn handle(
            &self,
            _event: &mut Event,
        ) -> Result<EventAction, synaptic_core::SynapticError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            Ok(EventAction::Continue)
        }
    }

    #[tokio::test]
    async fn parallel_event_fires_all_subscribers() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicU32::new(0));
        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: count.clone(),
                kind: EventKind::GatewayStart,
            }),
            0,
            "t1",
        );
        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: count.clone(),
                kind: EventKind::GatewayStart,
            }),
            0,
            "t2",
        );
        let mut event = Event::new(EventKind::GatewayStart, serde_json::Value::Null);
        let result = bus.emit(&mut event).await.unwrap();
        assert!(matches!(result, EmitResult::Proceed));
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    struct CancellingSubscriber;

    #[async_trait::async_trait]
    impl EventSubscriber for CancellingSubscriber {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(EventKind::BeforePromptBuild)]
        }
        async fn handle(&self, _: &mut Event) -> Result<EventAction, synaptic_core::SynapticError> {
            Ok(EventAction::Cancel)
        }
    }

    #[tokio::test]
    async fn sequential_event_can_be_cancelled() {
        let bus = EventBus::new();
        bus.subscribe(Arc::new(CancellingSubscriber), 0, "blocker");
        let mut event = Event::new(EventKind::BeforePromptBuild, serde_json::Value::Null);
        let result = bus.emit(&mut event).await.unwrap();
        assert!(matches!(result, EmitResult::Cancelled));
    }

    struct InterceptingSubscriber;

    #[async_trait::async_trait]
    impl EventSubscriber for InterceptingSubscriber {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(EventKind::BeforeToolCall)]
        }
        async fn handle(&self, _: &mut Event) -> Result<EventAction, synaptic_core::SynapticError> {
            Ok(EventAction::Intercept(serde_json::json!({"blocked": true})))
        }
    }

    #[tokio::test]
    async fn intercept_event_short_circuits() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicU32::new(0));
        // Interceptor at priority -100 (runs first) subscribes to BeforeToolCall.
        bus.subscribe(Arc::new(InterceptingSubscriber), -100, "interceptor");
        // Counter also subscribes to BeforeToolCall at priority 0 (runs second,
        // but should be skipped due to the intercept above).
        bus.subscribe(
            Arc::new(CountingSubscriber {
                count: count.clone(),
                kind: EventKind::BeforeToolCall,
            }),
            0,
            "counter",
        );
        let mut event = Event::new(EventKind::BeforeToolCall, serde_json::Value::Null);
        let result = bus.emit(&mut event).await.unwrap();
        assert!(matches!(result, EmitResult::Intercepted(_)));
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    struct RetrySubscriber;

    #[async_trait::async_trait]
    impl EventSubscriber for RetrySubscriber {
        fn subscriptions(&self) -> Vec<EventFilter> {
            vec![EventFilter::Exact(EventKind::OnToolError)]
        }
        async fn handle(&self, _: &mut Event) -> Result<EventAction, synaptic_core::SynapticError> {
            Ok(EventAction::Retry)
        }
    }

    #[tokio::test]
    async fn error_path_event_can_retry() {
        let bus = EventBus::new();
        bus.subscribe(Arc::new(RetrySubscriber), 0, "retrier");
        let mut event = Event::new(EventKind::OnToolError, serde_json::json!({"attempt": 1}));
        let result = bus.emit(&mut event).await.unwrap();
        assert!(matches!(result, EmitResult::Retry));
    }

    #[tokio::test]
    async fn subscribers_sorted_by_priority() {
        let bus = EventBus::new();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));

        struct OrderTracker {
            id: u32,
            order: Arc<std::sync::Mutex<Vec<u32>>>,
        }

        #[async_trait::async_trait]
        impl EventSubscriber for OrderTracker {
            fn subscriptions(&self) -> Vec<EventFilter> {
                vec![EventFilter::Exact(EventKind::BeforePromptBuild)]
            }
            async fn handle(
                &self,
                _: &mut Event,
            ) -> Result<EventAction, synaptic_core::SynapticError> {
                self.order.lock().unwrap().push(self.id);
                Ok(EventAction::Continue)
            }
        }

        bus.subscribe(
            Arc::new(OrderTracker {
                id: 3,
                order: order.clone(),
            }),
            10,
            "last",
        );
        bus.subscribe(
            Arc::new(OrderTracker {
                id: 1,
                order: order.clone(),
            }),
            -100,
            "first",
        );
        bus.subscribe(
            Arc::new(OrderTracker {
                id: 2,
                order: order.clone(),
            }),
            0,
            "middle",
        );

        let mut event = Event::new(EventKind::BeforePromptBuild, serde_json::Value::Null);
        bus.emit(&mut event).await.unwrap();
        assert_eq!(*order.lock().unwrap(), vec![1, 2, 3]);
    }
}
