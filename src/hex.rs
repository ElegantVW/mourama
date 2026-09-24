//! Axial hex helpers. Pointy-top, distance, rings, pixel.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hex {
    pub q: i32,
    pub r: i32,
}

impl Hex {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub fn s(self) -> i32 {
        -self.q - self.r
    }

    pub fn distance(self, other: Hex) -> i32 {
        let dq = (self.q - other.q).abs();
        let dr = (self.r - other.r).abs();
        let ds = (self.s() - other.s()).abs();
        (dq + dr + ds) / 2
    }

    #[allow(dead_code)]
    pub fn pixel(self, size: f64) -> (f64, f64) {
        let x = size * 3.0_f64.sqrt() * (self.q as f64 + self.r as f64 / 2.0);
        let y = size * (3.0 / 2.0) * self.r as f64;
        (x, y)
    }
}

pub fn disk(radius: i32) -> Vec<Hex> {
    let mut out = Vec::new();
    for q in -radius..=radius {
        let r1 = (-radius).max(-q - radius);
        let r2 = radius.min(-q + radius);
        for r in r1..=r2 {
            out.push(Hex::new(q, r));
        }
    }
    out
}

/// Five seats on ring `ring`, equally spaced around the 6*ring hexes.
pub fn seats_on_ring(ring: i32) -> [Hex; 5] {
    let ring_hexes = ring_cells(ring);
    let n = ring_hexes.len();
    let mut seats = [Hex::new(0, 0); 5];
    for i in 0..5 {
        seats[i] = ring_hexes[(i * n) / 5];
    }
    seats
}

fn ring_cells(ring: i32) -> Vec<Hex> {
    if ring == 0 {
        return vec![Hex::new(0, 0)];
    }
    let dirs = [
        Hex::new(1, 0),
        Hex::new(0, 1),
        Hex::new(-1, 1),
        Hex::new(-1, 0),
        Hex::new(0, -1),
        Hex::new(1, -1),
    ];
    let mut hex = Hex::new(dirs[4].q * ring, dirs[4].r * ring);
    let mut out = Vec::with_capacity((6 * ring) as usize);
    for dir in dirs {
        for _ in 0..ring {
            out.push(hex);
            hex = Hex::new(hex.q + dir.q, hex.r + dir.r);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_distance_is_radius() {
        for h in disk(5) {
            assert!(h.distance(Hex::new(0, 0)) <= 5);
        }
        assert_eq!(disk(5).len(), 91);
    }

    #[test]
    fn five_seats_are_distinct_and_on_ring() {
        let seats = seats_on_ring(4);
        let mut seen = std::collections::HashSet::new();
        for s in seats {
            assert_eq!(s.distance(Hex::new(0, 0)), 4);
            assert!(seen.insert((s.q, s.r)));
        }
    }
}
