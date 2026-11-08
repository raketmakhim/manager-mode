//! Weighted choice of a player from a lineup, shared by the models that decide who shoots, assists or fouls.
use super::MatchPlayer;
use crate::rng::Rng;

/// The index of a player chosen at random with chances in proportion to `weight`, except that nobody gets more than
/// `max_share` of the chances (the rest is shared by the others in proportion). If every weight is 0, the first.
pub(super) fn pick(players: &[MatchPlayer; 11], weight: impl Fn(&MatchPlayer) -> f64, max_share: f64, rng: &mut Rng) -> usize {
    let mut weights = players.each_ref().map(weight);
    // Cutting one player down can push another over the limit, so go round once per player (enough to settle).
    for _ in 0..weights.len() {
        let total: f64 = weights.iter().sum();
        for w in weights.iter_mut().filter(|w| **w > max_share * total) {
            *w = max_share / (1.0 - max_share) * (total - *w);
        }
    }
    let mut left = rng.unit() * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        left -= w;
        if left < 0.0 {
            return i;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::MatchTeam;

    #[test]
    fn nobody_gets_more_than_the_share_limit_even_with_several_far_above_the_rest() {
        let team = MatchTeam::flat(70);
        // Three slots carry nearly all the weight; the limit must still hold for each of them.
        let weight = |p: &MatchPlayer| if p.id >= 8 { 100.0 } else { 1.0 };
        let mut rng = Rng::new(11);
        let mut picks = [0u32; 11];
        for _ in 0..20_000 {
            picks[pick(&team.players, weight, 0.3, &mut rng)] += 1;
        }
        for (slot, n) in picks.iter().enumerate() {
            assert!(*n as f64 / 20_000.0 <= 0.3 + 0.02, "slot {slot} got {n} of 20000");
        }
    }

    #[test]
    fn with_every_weight_zero_the_first_slot_is_picked() {
        let team = MatchTeam::flat(70);
        assert_eq!(pick(&team.players, |_| 0.0, 0.3, &mut Rng::new(1)), 0);
    }
}
