//! Career reports, outcome sets and corner tables.
use crate::career::clock::{now_ms, prof};
use crate::career::inherit::{ExactOptions, Inheritance, aff_factor, inherit, set_ews};
use crate::career::pool::Pool;
use crate::career::schedule::{ExpectedRecord, PinkOutcome, ScheduleOutfits, apt_pink, pink_p};
use crate::career::search::Found;
use crate::career::{Ancestor, CareerSetup, DeckSearchMemo, Legacy, Lineage, SearchState};
use crate::deck::joint::exact::{JointScratch, joint_eval_in, joint_prep};
use crate::deck::joint::{JointExt, JointOptions};
use crate::deck::sources::TargetSources;
use crate::deck::space::SelectedDeck;
use crate::skills::{GEN_GOLD, OutfitOption};
use serde_json::{Value, json};
/// What a corner table (`CareerSetup::corners_kept`) reads besides the model: the trainee's deck class and the deck's
/// entries and filler cards.
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct CornerKey {
    /// `CareerSetup::deck_class` of the trainee.
    pub(crate) class: crate::uma::UmaIndex,
    pub(crate) entries: Vec<usize>,
    pub(crate) filler_cards: usize,
}

/// Multilinear interpolation of corner values `corners` (index bit k = coordinate k at 1) at `point`, with `buffer` the
/// caller keeps (the simulator and the plan evaluate tables millions of times).
/// Folds coordinate 0 first, halving the table each step: entry i after k folds is the full sweep's entry i << k (the
/// same multiply-adds on the same values, so the same bits), and only those reach entry 0.
pub(crate) fn interp_in(corners: &[f64], point: &[f64], buffer: &mut Vec<f64>) -> f64 {
    buffer.clear();
    buffer.extend_from_slice(corners);
    let mut n = buffer.len();
    for &coordinate in point {
        n /= 2;
        for i in 0..n {
            buffer[i] = buffer[2 * i] * (1.0 - coordinate) + buffer[2 * i + 1] * coordinate;
        }
    }
    buffer[0]
}

/// The outfits a career's report names (`trainee.outfits`) when it runs outfit option `option` with her schedule at
/// grades `schedule_grades` (`trainee.schedule_outfit`'s): the option's outfits at those grades, the schedule's first.
/// None ("any outfit") only where the option covers no target beyond `default_coverage` and every outfit of her
/// character has the same grades (`Uma::outfit_aptitudes` empty): an outfit with grades of its own is always named,
/// even as the only one left after exclusions.
fn named_outfits<'option>(
    uma: &crate::data::Uma,
    option: &'option OutfitOption,
    default_coverage: &[u8],
    schedule_grades: &[u8],
) -> Vec<&'option (u32, String)> {
    if option.coverage == default_coverage && uma.outfit_aptitudes.is_empty() {
        return vec![];
    }
    option.outfits.iter().filter(|card| uma.aptitudes_of(card.0) == schedule_grades).collect()
}

impl CareerSetup<'_> {
    /// One career's report: trainee and outfit, parents, the six legacy slots, per target odds, aptitudes, deck, schedule.
    pub(crate) fn report(&self, state: &mut SearchState, pool: &Pool, found: &Found, with_schedule: bool) -> Value {
        let data = self.data;
        let target_count = self.target_count;
        let uncertain_count = self.uncertain_count;
        let name = |id: u32| {
            data.umas.iter().find(|uma| uma.id == id).map_or_else(|| format!("Uma {id}"), |uma| uma.name.clone())
        };
        let lineage = found.lineage();
        let (trainee, first_parent, second_parent) = (found.trainee, found.first_parent, found.second_parent);
        let trainee_id = data.umas[trainee].id;
        let legacy_slots = self.slots(pool, lineage);
        let Inheritance { chances: q, carriers } = inherit(&legacy_slots, target_count);
        self.set_trainee(state, trainee, &q);
        let exact = self.exact(
            state,
            trainee,
            &found.deck,
            &carriers,
            ExactOptions { all_but_one: true, landed_sets: with_schedule && target_count <= 12 },
        );
        let (outfit, extras) = (exact.outfit, &exact.extras);
        let mut target_chances = vec![0.0f64; target_count];
        for (pos, &i) in self.uncertain_targets.iter().enumerate() {
            target_chances[i] = extras.marginals[outfit * uncertain_count + pos];
        }
        for (pos, &i) in self.fixed_targets.iter().enumerate() {
            target_chances[i] = GEN_GOLD * extras.scales.guaranteed[pos];
        }
        let gold_chances = self.gold_chances(state, trainee, &found.deck, &carriers, outfit);
        self.set_trainee(state, trainee, &q);
        // the outfit her schedule runs: her option's best for it (`plan_schedule`; solved here only where the option's
        // outfits' grades differ, or for the schedule's rows)
        let option_grades = self.outfit_grades(trainee, ScheduleOutfits::DeckOption(outfit));
        let planned = (with_schedule || option_grades.len() > 1)
            .then(|| self.plan_schedule(pool, lineage, ScheduleOutfits::DeckOption(outfit)));
        let schedule_outfit = planned.as_ref().map_or(option_grades[0], |planned| planned.outfit);
        let skill_job = self.skill_job;
        let OutfitOption { coverage, .. } = &self.model.outfit_options[trainee][outfit];
        let covers: Vec<Value> = coverage
            .iter()
            .zip(&self.uncertain_targets)
            .filter(|(level, i)| **level > 0 && !(**level == 1 && skill_job.targets[**i].guaranteed == 1))
            .map(|(level, i)| json!({"group": skill_job.targets[*i].group, "level": level}))
            .collect();
        let default_coverage: Vec<u8> =
            self.uncertain_targets.iter().map(|&i| u8::from(skill_job.targets[i].guaranteed == 1)).collect();
        let outfits: Vec<Value> = named_outfits(
            &data.umas[trainee],
            &self.model.outfit_options[trainee][outfit],
            &default_coverage,
            schedule_outfit.aptitudes,
        )
        .iter()
        .map(|card| json!({"id": card.0, "label": card.1}))
        .collect();
        let slots: Vec<Value> = legacy_slots.iter().map(|slot| json!({"kind": if slot.parent { "parent" } else { "grandparent" }, "legacy": slot.legacy, "gp": slot.grandparent_slot,
            "chara": slot.character_id, "name": name(slot.character_id), "rel": slot.relation_points, "race": slot.race, "aff": slot.relation_points + slot.race, "stars": &*slot.stars, "pinks": &*slot.pinks})).collect();
        // per target, buying it and landing it (MODEL.md, skills): bought as gold (her outfit's gold, or a deck gold hint)
        // or white (her outfit's white; else not gold and hinted by the deck, her own events or inheritance), each landing
        // with GEN_GOLD / GEN_WHITE x her spark scale. The gold chance is the exact one over the deck's chain states
        // (`gold_chances`), and the white part of `p` is the rest of it, so land_gold + land_white = p (to rounding); the
        // deck's own per-target hint chances (`found.deck.white_hint_chances`, `found.deck.gold_hint_chances`, the
        // product of its sources at their marginal chances) are shown as its sources. The scenario's golds: bought,
        // landing GEN_GOLD.
        let scales = self.scale(&carriers);
        let outfit_event_chances = &self.model.event_chances[trainee][outfit];
        let buy = |i: usize| -> Value {
            if let Some(pos) = self.uncertain_targets.iter().position(|&target| target == i) {
                let (coverage_level, white_hint_chances, gold_hint_chances, e) = (
                    coverage[pos],
                    found.deck.white_hint_chances[pos],
                    found.deck.gold_hint_chances[pos],
                    outfit_event_chances[pos],
                );
                let scale = scales.uncertain[pos];
                let gold = if coverage_level == 2 { 1.0 } else { gold_chances[pos] };
                let land_gold = gold * crate::skills::GEN_GOLD * scale;
                let land_white = if coverage_level == 2 { 0.0 } else { target_chances[i] - land_gold };
                let white = land_white / (crate::skills::GEN_WHITE * scale);
                json!({"gold": gold, "white": white, "land_gold": land_gold, "land_white": land_white,
                       "deck_gold": gold_hint_chances, "deck_white": white_hint_chances, "own_event": e, "inherit": q[i], "outfit": coverage_level})
            } else {
                let pos = self.fixed_targets.iter().position(|&target| target == i).unwrap_or(0);
                json!({"gold": 1.0, "white": 0.0, "land_gold": crate::skills::GEN_GOLD * scales.guaranteed.get(pos).copied().unwrap_or(1.0), "land_white": 0.0, "scenario": true})
            }
        };
        let targets: Vec<Value> = (0..target_count)
            .map(|i| {
                json!({"group": skill_job.targets[i].group, "q": q[i], "carriers": carriers[i], "p": target_chances[i],
            "gold": skill_job.targets[i].guaranteed == 2, "buy": buy(i)})
            })
            .collect();
        let display_deck = self.space.pdeck(&found.deck, &[trainee_id]);
        // pink sparks: per aptitude the chance at least one of the two inheritance rolls passes it on (1/3/5% per star)
        let mut pink_groups: Vec<u32> =
            legacy_slots.iter().flat_map(|slot| slot.pinks.iter().map(|spark| spark.0)).collect();
        pink_groups.sort_unstable();
        pink_groups.dedup();
        let pinks: Vec<Value> = pink_groups
            .iter()
            .map(|&group| {
                let (mut miss, mut n) = (1.0f64, 0);
                for slot in &legacy_slots {
                    if let Some(stars) =
                        slot.pinks.iter().filter(|spark| spark.0 == group).map(|spark| spark.1.min(3)).max()
                        && stars > 0
                    {
                        n += 1;
                        miss *= 1.0 - pink_p(stars, aff_factor(slot.relation_points, slot.race));
                    }
                }
                json!({"group": group, "q": 1.0 - miss * miss, "carriers": n})
            })
            .collect();
        // her aptitudes in the schedule's outfit and her veteran's pink spark (MODEL.md): base and start grades (data.json
        // `apt` order, codes 1..8 = G..S), each legacy pink's chance to fire at one April inspiration, and the chance of
        // each pink type; with a veteran before her result is known among her legacies, `outcomes` (each one's chance,
        // start grades and sparks) in place of `start` and `fire`
        let unplanned_pink;
        let pink_career = if let Some(planned) = &planned {
            planned.pink.as_ref()
        } else {
            unplanned_pink = self.pink_career(pool, lineage, schedule_outfit.aptitudes);
            unplanned_pink.as_ref()
        };
        let aptitudes = pink_career.map(|pink| {
            let pink_gen = Self::pink_gen(pink);
            let fire_json = |outcome: &PinkOutcome| -> Vec<Value> {
                outcome.fire.iter().map(|spark| json!({"group": apt_pink(spark.aptitude), "apt": spark.aptitude, "p": spark.chance, "slot": spark.slot, "stars": spark.stars})).collect()
            };
            let mut aptitudes = json!({"base": pink.base,
                   "pink_gen": (0..10).map(|k| json!({"group": apt_pink(k), "p": pink_gen[k]})).collect::<Vec<_>>()});
            if let [outcome] = pink.outcomes.as_slice() {
                aptitudes["start"] = json!(outcome.start);
                aptitudes["fire"] = json!(fire_json(outcome));
            } else {
                aptitudes["outcomes"] = pink.outcomes.iter().map(|outcome| json!({"p": outcome.chance, "start": outcome.start, "fire": fire_json(outcome)})).collect();
            }
            aptitudes
        });
        let outcomes =
            if with_schedule && target_count <= 12 { json!(self.outcome_sets(outfit, extras)) } else { Value::Null };
        let (schedule, win_chances) = match &planned {
            Some(planned) if with_schedule => {
                (self.schedule_rows(pool, lineage, planned), planned.win_chances.as_slice())
            }
            _ => (Value::Null, [].as_slice()),
        };
        json!({"trainee": {"id": trainee_id, "name": data.umas[trainee].name, "outfits": outfits, "covers": covers, "schedule_outfit": schedule_outfit.card},
               "parents": [first_parent, second_parent], "slots": slots, "targets": targets, "p_all": exact.p_all, "p_minus1": exact.p_all_but_one, "expected": target_chances.iter().sum::<f64>(),
               "bound": found.ceiling, "deck": display_deck.out(&self.uncertain_groups), "pinks": pinks, "aptitudes": aptitudes, "schedule": schedule, "outcome_sets": outcomes, "g1_wins": if with_schedule { json!(win_chances.iter().sum::<f64>()) } else { Value::Null }})
    }

    /// Corner table of trainee t with deck `deck`: per outfit, P(all) with every spark-generation scale 1 at the 2^nu
    /// points q_k = 1 - (1 - q0_k) x_k, x_k in {0, 1} (index bit k = uncertain target k at x_k = 1). Derived: q_k
    /// enters every landing factor only through (1 - e)(1 - q_k), affinely, and P(all) per outfit is a sum of products
    /// with one factor per target, so it is multilinear in x and `interp_in` at any x in [0, 1]^nu gives it exactly.
    /// `corners` at q0 = 0 (a cached deck's table, `Cache::add`), memoised by `CornerKey`. Exact: the
    /// table reads trainee t only through her model (outfit coverage, own-skill chances: `set_trainee` from
    /// `Model::event_chances`), which her deck class holds to the bit, and the deck only through its entries and
    /// filler; the memo is kept between jobs of one setup (`KeptState::decks`: the same model and entries). `corners`
    /// leaves her folded buffer at its last corner, q = 0 for every target (1 - (1 - 0) x 1); a hit sets the same state
    pub(crate) fn corners_kept(
        &self,
        state: &mut SearchState,
        memo: &mut DeckSearchMemo,
        trainee: crate::uma::UmaIndex,
        deck: &SelectedDeck,
    ) -> Vec<Vec<f64>> {
        let key = CornerKey {
            class: self.deck_class[trainee],
            entries: deck.entries.clone(),
            filler_cards: deck.filler_cards,
        };
        let zero_chances = vec![0.0; self.target_count];
        if let Some(table) = memo.corners.get(&key) {
            let table = (**table).clone();
            self.set_trainee(state, trainee, &zero_chances);
            return table;
        }
        let start_ms = now_ms();
        let table = self.corners(state, trainee, deck, &zero_chances);
        prof("corner_ms", now_ms() - start_ms);
        prof("corner_builds", 1.0);
        memo.corners.insert(key, std::rc::Rc::new(table.clone()));
        table
    }
    fn corners(
        &self,
        state: &mut SearchState,
        trainee: crate::uma::UmaIndex,
        deck: &SelectedDeck,
        base_chances: &[f64],
    ) -> Vec<Vec<f64>> {
        let (uncertain_count, target_count) = (self.uncertain_count, self.target_count);
        let outfit_count = self.model.n_outfits(trainee);
        let mut corners = vec![vec![0.0f64; 1 << uncertain_count]; outfit_count];
        // The exact odds per corner at scale 1: prepare the deck once (`joint_prep`, which reads no chance or model
        // data), then evaluate it per corner with the same sums as `exact`.
        let scales = self.scale(&vec![6; target_count]);
        let entries: Vec<crate::deck::sources::SourceEntry<&TargetSources>> = deck
            .entries
            .iter()
            .map(|&j| crate::deck::sources::SourceEntry { sources: &self.space.sources[j], kind: self.space.kind[j] })
            .collect();
        let chain_events = self.deck_events(trainee, deck);
        let prepared = joint_prep(chain_events, self.model.uncertain.len(), &entries, deck.filler_cards);
        // (one chance vector and one `JointExt` for every corner: `JointPrep::evaluate` only reads `scales`; `marginals` is
        // not read here, so not summed)
        let mut q = base_chances.to_vec();
        let mut ext = JointExt { scales, marginals: vec![], sets: vec![], per_outfit: vec![] };
        // (and one set of `JointPrep::evaluate`'s buffers)
        let mut scratch = JointScratch::default();
        for corner in 0..(1usize << uncertain_count) {
            for (k, &i) in self.uncertain_targets.iter().enumerate() {
                let coordinate = if corner >> k & 1 == 1 { 1.0 } else { 0.0 };
                q[i] = 1.0 - (1.0 - base_chances[i]) * coordinate;
            }
            set_ews(&mut state.folded_events[trainee], &self.model.event_chances[trainee], &self.uncertain_targets, &q);
            joint_eval_in(
                self.model,
                self.joint_query(state, trainee, chain_events),
                &prepared,
                JointOptions { target_count, all_but_one: false, marginals: false },
                Some(&mut ext),
                &mut scratch,
            );
            for outfit in 0..outfit_count {
                corners[outfit][corner] = ext.per_outfit[outfit];
            }
        }
        corners
    }

    /// Joint distribution of full target sets, including guaranteed gold generation.
    pub(crate) fn outcome_sets(&self, outfit: usize, extras: &JointExt) -> Vec<f64> {
        let uncertain_sets = &extras.sets[(outfit << self.uncertain_count)..((outfit + 1) << self.uncertain_count)];
        let mut set_chances = vec![0.0f64; 1 << self.target_count];
        for (uncertain_mask, &chance) in uncertain_sets.iter().enumerate() {
            if chance == 0.0 {
                continue;
            }
            let mut base = 0usize;
            for (k, &i) in self.uncertain_targets.iter().enumerate() {
                if uncertain_mask >> k & 1 == 1 {
                    base |= 1 << i;
                }
            }
            let mut part = vec![(base, chance)];
            for (pos, &i) in self.fixed_targets.iter().enumerate() {
                let gold_chance = GEN_GOLD * extras.scales.guaranteed[pos];
                part = part
                    .into_iter()
                    .flat_map(|(mask, weight)| {
                        [(mask, weight * (1.0 - gold_chance)), (mask | 1 << i, weight * gold_chance)]
                    })
                    .collect();
            }
            for (mask, weight) in part {
                set_chances[mask] += weight;
            }
        }
        set_chances
    }

    /// The veteran the lineage's career leaves, before her result is known: the trainee, a spark of every target with
    /// stars drawn from the rank mix (`star_dist`), her schedule's win chances, her possible pinks (her pink spark as
    /// the mixture `pink_career` expands for her children: exact, one at most), and the lineage's parents; from her
    /// schedule's `record`.
    pub(crate) fn veteran(&self, legacies: &[Legacy], lineage: Lineage, record: &ExpectedRecord) -> Legacy {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        Legacy {
            character_id: self.data.umas[trainee].id,
            borrowed: false,
            sparks: self.skill_job.targets.iter().map(|target| (target.group, 2)).collect(),
            wins: vec![],
            pinks: vec![],
            parents: vec![Ancestor::of_legacy(&legacies[first_parent]), Ancestor::of_legacy(&legacies[second_parent])],
            // (the veteran owns her copies: the record stays in the memo)
            win_chances: record.win_chances.to_vec(),
            mix: Some(self.skill_job.star_dist),
            pink_chances: record.pink_chances.to_vec(),
        }
    }

    /// Per uncertain target, her exact chance of buying it as gold with deck `deck` in outfit option `outfit` (her
    /// lineage's `carriers`): a target's landing factor per chain state and outcome is GG g + GW (1 - g - n (1 - e)(1 -
    /// q)) (`LandPre`; g = 1 at a coupled gold hint), so at q = 1 its marginal is s (GG E[g] + GW (1 - E[g])), and E[g]
    /// follows from one evaluation there (to rounding). Leaves her folded buffer at q = 1.
    fn gold_chances(
        &self,
        state: &mut SearchState,
        trainee: crate::uma::UmaIndex,
        deck: &SelectedDeck,
        carriers: &[i32],
        outfit: usize,
    ) -> Vec<f64> {
        use crate::skills::{GEN_GOLD, GEN_WHITE};
        self.set_trainee(state, trainee, &vec![1.0; self.target_count]);
        let at_certain_inheritance =
            self.exact(state, trainee, deck, carriers, ExactOptions { all_but_one: false, landed_sets: false });
        let scales = self.scale(carriers);
        (0..self.uncertain_count)
            .map(|pos| {
                let landing = at_certain_inheritance.extras.marginals[outfit * self.uncertain_count + pos];
                (landing / scales.uncertain[pos] - GEN_WHITE) / (GEN_GOLD - GEN_WHITE)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::named_outfits;
    use crate::skills::OutfitOption;

    #[test]
    fn an_outfit_with_its_own_grades_is_always_named() {
        let data = crate::data::load(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"));
        let uma_of = |id: u32| data.umas.iter().find(|uma| uma.id == id).unwrap();
        // Haru Urara with [Bestest Prize ♪] excluded and no target covered: [New Year ♪ New Urara!] (mile A, her
        // own) is named, not "any outfit"
        let haru_urara = uma_of(1052);
        let new_year = OutfitOption { coverage: vec![0], outfits: vec![(105_202, "[New Year ♪ New Urara!]".into())] };
        let grades = haru_urara.aptitudes_of(105_202);
        assert_ne!(grades, haru_urara.aptitudes.as_slice());
        let named: Vec<u32> = named_outfits(haru_urara, &new_year, &[0], grades).iter().map(|card| card.0).collect();
        assert_eq!(named, [105_202]);
        // an uma whose outfits share grades, covering nothing: any outfit
        let oguri_cap = uma_of(1006);
        assert!(oguri_cap.outfit_aptitudes.is_empty());
        let any = OutfitOption { coverage: vec![0], outfits: vec![(100_601, "[Starlight Beat]".into())] };
        assert!(named_outfits(oguri_cap, &any, &[0], &oguri_cap.aptitudes).is_empty());
    }
}
