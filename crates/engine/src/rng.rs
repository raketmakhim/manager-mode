//! A small seeded random number generator (SplitMix64). Our own, so a seed gives the same
//! numbers on every machine and library version, which keeps saved games and tests repeatable.

/// Random numbers from a seed. Pass `&mut Rng` into anything random; never create one inside a sim.
pub struct Rng(u64);

impl Rng {
    /// The same seed always gives the same sequence.
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 to `n` - 1. The modulo bias is about n / 2^64, far too small to matter here.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// Puts the items in a random order (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            items.swap(i, self.below(i + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_seed_gives_the_same_shuffle() {
        let shuffled = |seed| {
            let mut v: Vec<u32> = (0..20).collect();
            Rng::new(seed).shuffle(&mut v);
            v
        };
        assert_eq!(shuffled(1), shuffled(1));
        assert_ne!(shuffled(1), shuffled(2));
    }

    #[test]
    fn a_shuffle_keeps_every_item() {
        let mut v: Vec<u32> = (0..50).collect();
        Rng::new(9).shuffle(&mut v);
        v.sort();
        assert_eq!(v, (0..50).collect::<Vec<_>>());
    }
}
