//! Learning from the user's choices (spec §10.11): opt-in, on the device only. A small table of
//! counts per scenario ([`Weight`]) turns into gentle multipliers on the scenario weights.
//!
//! Signals, recorded by the engine itself when a prediction is committed:
//! - **override**: a lower prediction was chosen (↓ or a click) over the top one;
//! - **accept**: the top one was committed while other kinds were shown;
//! - **regret**: Backspace right after an accept (weak).
//!
//! Only kinds that were on screen count (no exposure bias). Nothing else is kept: no text, no
//! keys, no times. Updates are pure and deterministic; hosts store [`Learner::export`] and give it
//! back with [`Learner::import`].

use crate::predict::{Kind, Weight};

const KINDS: usize = Weight::ALL.len();
/// Scenarios in the first export format (before Implied ㄹ): the fewest columns an import takes.
const FIRST_KINDS: usize = 7;

/// Pseudo-counts on each side of a rate: about this many events before a weight moves much.
const PRIOR: f64 = 100.0;
/// How often a kind shown below the top is chosen, before any learning.
const PROMOTE_RATE: f64 = 0.05;
/// How often a top kind is passed over (or regretted), before any learning.
const DEMOTE_RATE: f64 = 0.05;
/// A regret counts this much of a demotion.
const REGRET: f64 = 0.5;
/// How strongly the learned log-odds move a weight.
const GAIN: f64 = 0.35;
/// Learned weights stay within these (a user's own weights may go 0.1–10).
pub const MIN: f64 = 0.5;
pub const MAX: f64 = 2.0;
/// Counts halve every this many events, so habits can change.
const HALF_LIFE: f64 = 400.0;

/// The scenario a prediction's kind is tuned by (the bare-vowel reading by Implied ㅇ, inversely:
/// see [`Learner::weight`]).
pub fn scenario(kind: Kind) -> Weight {
    match kind {
        Kind::Korean => Weight::Korean,
        Kind::English | Kind::EnglishCapitalized | Kind::EnglishUpper => Weight::English,
        Kind::BareVowels => Weight::Implied,
        Kind::Mixed => Weight::Mixed,
        Kind::TwoSet => Weight::TwoSet,
        Kind::KoreanKeepFinals => Weight::KeepFinals,
        Kind::Hanja => Weight::Hanja,
        Kind::ImpliedL => Weight::ImpliedL,
    }
}

/// What the user did with a list of predictions.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Committed prediction `chosen` of the kinds shown (top first).
    Chose { shown: Vec<Weight>, chosen: usize },
    /// Backspace right after accepting a top prediction of this kind.
    Regret(Weight),
}

/// The counts, per scenario.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Learner {
    /// Chosen while shown below the top.
    promoted: [f64; KINDS],
    /// Shown below the top while the top was accepted.
    passed: [f64; KINDS],
    /// Top, but something below was chosen (or regretted).
    demoted: [f64; KINDS],
    /// Top and accepted, with other kinds shown.
    confirmed: [f64; KINDS],
    /// Events since the last halving.
    events: f64,
}

impl Learner {
    /// The counts after `event`. Pure: the learner itself doesn't change.
    pub fn after(&self, event: &Event) -> Learner {
        let mut next = self.clone();
        match event {
            Event::Chose { shown, chosen } => {
                let mut kinds: Vec<Weight> = Vec::new();
                for &k in shown {
                    if !kinds.contains(&k) {
                        kinds.push(k);
                    }
                }
                let (Some(&top), Some(&pick)) = (shown.first(), shown.get(*chosen)) else { return next };
                // Only a choice between different kinds says anything about the kinds.
                if kinds.len() < 2 || (pick == top && *chosen != 0) {
                    return next;
                }
                // A choice between the Korean reading and one of its alternatives (bare vowels,
                // kept finals, 2-set, English + particle, hanja) is that alternative's business;
                // only Korean against English moves those two.
                let base = |w: Weight| matches!(w, Weight::Korean | Weight::English);
                if pick == top {
                    if !base(top) || kinds.iter().any(|&k| k != top && base(k)) {
                        next.confirmed[top as usize] += 1.0;
                    }
                    for &k in kinds.iter().filter(|&&k| k != top && (!base(k) || base(top))) {
                        next.passed[k as usize] += 1.0;
                    }
                } else {
                    if !base(pick) || base(top) {
                        next.promoted[pick as usize] += 1.0;
                    }
                    if !base(top) || base(pick) {
                        next.demoted[top as usize] += 1.0;
                    }
                }
            }
            Event::Regret(kind) => next.demoted[*kind as usize] += REGRET,
        }
        next.events += 1.0;
        if next.events >= HALF_LIFE {
            for table in [&mut next.promoted, &mut next.passed, &mut next.demoted, &mut next.confirmed] {
                table.iter_mut().for_each(|n| *n /= 2.0);
            }
            next.events = 0.0;
        }
        next
    }

    /// The learned multiplier for a scenario, 1.0 with no evidence, within [`MIN`]–[`MAX`]. The
    /// counts for Implied ㅇ are about the bare-vowel reading (that's what's chosen or passed
    /// over), while its weight is how likely the implied ㅇ is: so it's the inverse.
    pub fn weight(&self, which: Weight) -> f64 {
        let raw = self.raw_weight(which);
        if which == Weight::Implied { 1.0 / raw } else { raw }
    }

    fn raw_weight(&self, which: Weight) -> f64 {
        let i = which as usize;
        // Log-odds of a rate against its prior, with PRIOR pseudo-counts at the prior rate.
        let shift = |hits: f64, misses: f64, rate: f64| {
            let p = (hits + PRIOR * rate) / (hits + misses + PRIOR);
            (p / (1.0 - p)).ln() - (rate / (1.0 - rate)).ln()
        };
        let up = shift(self.promoted[i], self.passed[i], PROMOTE_RATE);
        let down = shift(self.demoted[i], self.confirmed[i], DEMOTE_RATE);
        (GAIN * (up - down)).exp().clamp(MIN, MAX)
    }

    /// The counts as one line of text, for the host to store.
    pub fn export(&self) -> String {
        let mut fields = vec!["jkl1".to_string(), format!("{}", self.events)];
        for table in [&self.promoted, &self.passed, &self.demoted, &self.confirmed] {
            fields.push(table.iter().map(|n| format!("{n}")).collect::<Vec<_>>().join(","));
        }
        fields.join(" ")
    }

    /// Counts from [`Learner::export`]; None if the text isn't that.
    pub fn import(text: &str) -> Option<Learner> {
        let mut fields = text.split_whitespace();
        if fields.next()? != "jkl1" {
            return None;
        }
        let events: f64 = fields.next()?.parse().ok()?;
        let mut tables = [[0.0; KINDS]; 4];
        for table in &mut tables {
            let values: Vec<f64> = fields.next()?.split(',').map(str::parse).collect::<Result<_, _>>().ok()?;
            // Exports from before a scenario was added have fewer columns: the rest start at 0.
            if values.len() < FIRST_KINDS || values.len() > KINDS || values.iter().any(|v| !v.is_finite() || *v < 0.0) {
                return None;
            }
            table[..values.len()].copy_from_slice(&values);
        }
        let [promoted, passed, demoted, confirmed] = tables;
        Some(Learner { promoted, passed, demoted, confirmed, events: events.clamp(0.0, HALF_LIFE) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chose(shown: &[Weight], chosen: usize) -> Event {
        Event::Chose { shown: shown.to_vec(), chosen }
    }

    #[test]
    fn no_evidence_is_no_change() {
        let l = Learner::default();
        for w in Weight::ALL {
            assert!((l.weight(w) - 1.0).abs() < 1e-9);
        }
    }

    #[test]
    fn one_choice_barely_moves_and_many_converge_within_the_clamp() {
        let event = chose(&[Weight::Korean, Weight::Implied], 1);
        let once = Learner::default().after(&event);
        assert!(once.weight(Weight::Implied) > 0.9, "{}", once.weight(Weight::Implied));
        let mut l = Learner::default();
        for _ in 0..60 {
            l = l.after(&event);
        }
        // Bare vowels chosen: the implied ㅇ grows less likely.
        assert!(l.weight(Weight::Implied) < 0.56 && l.weight(Weight::Implied) >= MIN);
        // Picking bare vowels over the Korean reading says nothing about Korean against English.
        assert_eq!(l.weight(Weight::Korean), 1.0);
    }

    #[test]
    fn accepting_the_top_keeps_it_and_choices_within_a_kind_say_nothing() {
        let mut l = Learner::default();
        for _ in 0..50 {
            l = l.after(&chose(&[Weight::Korean, Weight::English], 0));
        }
        assert!(l.weight(Weight::Korean) >= 1.0 && l.weight(Weight::English) <= 1.0);
        let same = Learner::default().after(&chose(&[Weight::English, Weight::English], 1));
        assert_eq!(same, Learner::default());
    }

    #[test]
    fn counts_halve_and_round_trip_through_text() {
        let mut l = Learner::default();
        for _ in 0..(HALF_LIFE as usize) {
            l = l.after(&chose(&[Weight::Korean, Weight::Mixed], 1));
        }
        assert!(l.promoted[Weight::Mixed as usize] < HALF_LIFE * 0.6);
        assert_eq!(Learner::import(&l.export()), Some(l));
        assert_eq!(Learner::import("garbage"), None);
        assert_eq!(Learner::import("jkl1 1 1,2 3 4 5"), None);
        // An export from before Implied ㄹ (seven columns) still loads.
        let old = "jkl1 3 0,0,3,0,0,0,0 0,0,0,0,0,0,0 0,0,0,0,0,0,0 0,0,0,0,0,0,0";
        assert_eq!(Learner::import(old).map(|l| l.promoted[Weight::Implied as usize]), Some(3.0));
    }
}
