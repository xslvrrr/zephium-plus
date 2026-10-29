//! rice fork: named pointer zones over the window. A zone is reached inside
//! one of its activation rects and held while the pointer stays inside its
//! larger keep rect, so a small target at the edge opens something that a
//! pointer can then move around in without it closing.

use zephium_core::geometry::Rect;

#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub name: String,
    pub activate: Vec<Rect>,
    pub keep: Rect,
}

#[derive(Default)]
pub struct Tracker {
    zones: Vec<(Zone, bool)>,
}

fn contains(rect: &Rect, x: f64, y: f64) -> bool {
    x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
}

impl Tracker {
    /// Replaces the zones. A zone that keeps its name keeps its state, so
    /// re-measuring an open notch does not close it.
    pub fn set(&mut self, zones: Vec<Zone>) -> Vec<(String, bool)> {
        let mut left = Vec::new();
        let old = std::mem::take(&mut self.zones);
        for (zone, inside) in &old {
            if *inside && !zones.iter().any(|z| z.name == zone.name) {
                left.push((zone.name.clone(), false));
            }
        }
        self.zones = zones
            .into_iter()
            .map(|zone| {
                let inside = old.iter().any(|(z, inside)| *inside && z.name == zone.name);
                (zone, inside)
            })
            .collect();
        left
    }

    /// The zones whose state the pointer at `(x, y)` changes.
    pub fn moved(&mut self, x: f64, y: f64) -> Vec<(String, bool)> {
        let mut changed = Vec::new();
        for (zone, inside) in &mut self.zones {
            let now = if *inside {
                contains(&zone.keep, x, y)
            } else {
                zone.activate.iter().any(|rect| contains(rect, x, y))
            };
            if now != *inside {
                *inside = now;
                changed.push((zone.name.clone(), now));
            }
        }
        changed
    }

    /// The pointer left the window: every zone is left.
    pub fn exited(&mut self) -> Vec<(String, bool)> {
        self.zones
            .iter_mut()
            .filter(|(_, inside)| *inside)
            .map(|(zone, inside)| {
                *inside = false;
                (zone.name.clone(), false)
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> Zone {
        Zone {
            name: "notch".into(),
            activate: vec![Rect::new(100.0, 0.0, 200.0, 4.0)],
            keep: Rect::new(68.0, 0.0, 264.0, 82.0),
        }
    }

    #[test]
    fn reached_at_the_strip_and_held_in_the_keep() {
        let mut t = Tracker::default();
        t.set(vec![zone()]);
        assert!(t.moved(150.0, 40.0).is_empty());
        assert_eq!(t.moved(150.0, 2.0), vec![("notch".into(), true)]);
        assert!(t.moved(80.0, 70.0).is_empty());
        assert_eq!(t.moved(80.0, 90.0), vec![("notch".into(), false)]);
    }

    #[test]
    fn resetting_keeps_an_open_zone_open() {
        let mut t = Tracker::default();
        t.set(vec![zone()]);
        t.moved(150.0, 2.0);
        assert!(t.set(vec![zone()]).is_empty());
        assert_eq!(t.set(vec![]), vec![("notch".into(), false)]);
    }
}
