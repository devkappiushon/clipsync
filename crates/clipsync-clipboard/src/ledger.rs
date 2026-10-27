//! Own-write ledger (rule 2) and seen-events set (rule 5). Hashes live in memory only, never in logs.
use clipsync_proto::{EventId, SEEN_EVENTS_CAP};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

pub type Hash = [u8; 32];

pub const LEDGER_VALIDITY: Duration = Duration::from_secs(2);

pub fn hash_text(text: &str) -> Hash {
    Sha256::digest(text.as_bytes()).into()
}

#[derive(Default)]
pub struct WriteLedger {
    entries: VecDeque<(Hash, Instant)>,
}

impl WriteLedger {
    pub fn register(&mut self, h: Hash, now: Instant) {
        self.prune(now);
        self.entries.push_back((h, now));
    }
    /// Entries are not consumed: one write can raise several events (echo plus ownership takeover).
    pub fn is_echo(&mut self, h: &Hash, now: Instant) -> bool {
        self.prune(now);
        self.entries.iter().any(|(e, _)| e == h)
    }
    pub fn forget(&mut self, h: &Hash) {
        if let Some(i) = self.entries.iter().rposition(|(e, _)| e == h) {
            self.entries.remove(i);
        }
    }
    fn prune(&mut self, now: Instant) {
        while self
            .entries
            .front()
            .is_some_and(|(_, t)| now.duration_since(*t) > LEDGER_VALIDITY)
        {
            self.entries.pop_front();
        }
    }
}

pub struct SeenEvents {
    ttl: Duration,
    map: HashMap<EventId, Instant>,
    order: VecDeque<EventId>,
}

impl SeenEvents {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }
    /// True when `id` is new (and is now remembered); false for a repeat inside the TTL.
    pub fn insert_new(&mut self, id: EventId, now: Instant) -> bool {
        while let Some(front) = self.order.front().copied() {
            let expired = self
                .map
                .get(&front)
                .is_none_or(|t| now.duration_since(*t) > self.ttl);
            if expired || self.order.len() >= SEEN_EVENTS_CAP {
                self.order.pop_front();
                self.map.remove(&front);
            } else {
                break;
            }
        }
        if self.map.contains_key(&id) {
            return false;
        }
        self.map.insert(id, now);
        self.order.push_back(id);
        true
    }
}
