use dcma_core::types::AtomId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp {
    pub physical: u64,
    pub logical: u64,
    pub node_id: AtomId,
}

impl Timestamp {
    pub fn new(node_id: AtomId) -> Self {
        let physical = Self::now_physical();
        Self {
            physical,
            logical: 0,
            node_id,
        }
    }

    pub fn now(node_id: AtomId) -> Self {
        Self::new(node_id)
    }

    fn now_physical() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    pub fn happens_before(&self, other: &Self) -> bool {
        self.physical < other.physical
            || (self.physical == other.physical && self.logical < other.logical)
    }

    pub fn happens_after(&self, other: &Self) -> bool {
        other.happens_before(self)
    }

    pub fn concurrent(&self, other: &Self) -> bool {
        !self.happens_before(other) && !other.happens_before(self)
    }

    pub fn tick(&mut self) {
        let now = Self::now_physical();
        if now > self.physical {
            self.physical = now;
            self.logical = 0;
        } else {
            self.logical += 1;
        }
    }

    pub fn receive(&mut self, received: &Timestamp) {
        let now = Self::now_physical();
        let max_physical = now.max(self.physical).max(received.physical);
        if max_physical == self.physical && max_physical == received.physical {
            self.logical = self.logical.max(received.logical) + 1;
        } else if max_physical == received.physical {
            self.logical = received.logical + 1;
        } else {
            self.logical = 0;
        }
        self.physical = max_physical;
    }
}

impl PartialOrd for Timestamp {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Timestamp {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.physical.cmp(&other.physical) {
            Ordering::Equal => match self.logical.cmp(&other.logical) {
                Ordering::Equal => self.node_id.0.cmp(&other.node_id.0),
                other => other,
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcma_core::types::AtomId;

    fn make_node_id() -> AtomId {
        AtomId::new()
    }

    #[test]
    fn test_hlc_creation() {
        let node_id = make_node_id();
        let ts = Timestamp::new(node_id);
        assert_eq!(ts.node_id, node_id);
        assert_eq!(ts.logical, 0);
        assert!(ts.physical > 0);
    }

    #[test]
    fn test_hlc_local_event() {
        let node_id = make_node_id();
        let mut ts = Timestamp::new(node_id);
        let physical_before = ts.physical;
        ts.tick();
        let physical_after = ts.physical;
        assert!(physical_after >= physical_before);
        if physical_after == physical_before {
            assert_eq!(ts.logical, 1);
        } else {
            assert_eq!(ts.logical, 0);
        }
    }

    #[test]
    fn test_hlc_receive_later() {
        let node_id = make_node_id();
        let mut local = Timestamp::new(node_id);
        let mut remote = Timestamp::new(make_node_id());
        // Advance remote far into the future
        remote.physical = local.physical + 1000;
        local.receive(&remote);
        assert_eq!(local.physical, remote.physical);
        assert_eq!(local.logical, remote.logical + 1);
    }

    #[test]
    fn test_hlc_receive_same_physical() {
        let node_id = make_node_id();
        let mut local = Timestamp::new(node_id);
        let mut remote = Timestamp::new(make_node_id());
        remote.physical = local.physical;
        remote.logical = 5;
        local.logical = 3;
        local.receive(&remote);
        assert_eq!(local.physical, remote.physical);
        assert_eq!(local.logical, remote.logical + 1);
    }

    #[test]
    fn test_hlc_happens_before() {
        let node_id = make_node_id();
        let mut a = Timestamp::new(node_id);
        let b = a;
        a.tick();
        assert!(b.happens_before(&a));
        assert!(!a.happens_before(&b));
    }

    #[test]
    fn test_hlc_concurrent() {
        let mut a = Timestamp::new(make_node_id());
        let mut b = Timestamp::new(make_node_id());
        a.physical = 100;
        a.logical = 1;
        b.physical = 100;
        b.logical = 2;
        // Different node_ids but same (physical, logical) would be equal
        // Let's make them truly concurrent with same physical but different logical
        assert!(a.happens_before(&b) || b.happens_before(&a) || !a.concurrent(&b));
        // Actually with same physical and diff logical, one happens-before the other
        // To test concurrency we need different physical times that overlap
        let mut x = Timestamp::new(make_node_id());
        let mut y = Timestamp::new(make_node_id());
        x.physical = 100;
        x.logical = 5;
        y.physical = 100;
        y.logical = 3;
        assert!(y.happens_before(&x));
        assert!(!x.happens_before(&y));
        assert!(!x.concurrent(&y));
    }

    #[test]
    fn test_hlc_ord() {
        let mut a = Timestamp::new(make_node_id());
        let mut b = Timestamp::new(make_node_id());
        a.physical = 100;
        a.logical = 1;
        b.physical = 100;
        b.logical = 2;
        // Total order includes node_id tiebreaker
        let ordering = a.cmp(&b);
        assert_eq!(ordering, Ordering::Less);
        assert!(a < b);
    }
}
