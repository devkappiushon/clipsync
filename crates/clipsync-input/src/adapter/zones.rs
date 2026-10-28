//! Comparing zone lists by content.
use super::Zone;

/// True when both lists describe the same monitors: same name, position, size and scale for every
/// zone, in the same order (§13.8 item 1). A different count is a change too.
pub(crate) fn same_zones(a: &[Zone], b: &[Zone]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(p, q)| {
            p.index == q.index
                && p.name == q.name
                && p.x == q.x
                && p.y == q.y
                && p.width == q.width
                && p.height == q.height
                && p.scale.to_bits() == q.scale.to_bits()
        })
}
