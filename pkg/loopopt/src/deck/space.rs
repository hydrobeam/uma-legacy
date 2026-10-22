//! The space of legal decks (`DeckSpace`): entries by chain-kind key, legality tables and caps.
use crate::deck::caps::{AUDIT_CAP, EXACT_CAP, EXACT_CAPS, cap_class};
use crate::deck::chains::{ChainKind, chain_marginals};
use crate::deck::dominance::{Name, PDeck};
use crate::deck::joint::MissFactors;
use crate::deck::sources::{ChainOutcome, Entries, EventOutcome, Group, Sources, TargetSources};
use crate::deck::{Card, CardsRaw, DeckJob};
use crate::skills::{Model, OutfitOption, SkillsRaw};
use std::collections::HashMap;

/// A configuration's required cards from one chain-kind group.
#[derive(Clone, Copy)]
pub(crate) struct GroupSlots {
    /// Index in the search-wide group tables.
    pub(crate) group: usize,
    /// Number of cards to choose from this group.
    pub(crate) slots: usize,
}

/// A configured group with its legal entry indices borrowed for traversal.
#[derive(Clone, Copy)]
pub(crate) struct TraversalEntryGroup<'entries> {
    /// Entries in the unchanged pick order.
    pub(crate) entries: &'entries [usize],
    pub(crate) slots: usize,
}

/// An earlier entry that can replace a candidate under the product bound.
#[derive(Clone, Copy)]
pub(crate) struct Dominator {
    /// Position within the candidate's group.
    pub(crate) position: u32,
    /// Targets whose gold miss improves strictly; their rounding gaps must be checked.
    pub(crate) gold_gap_targets: u32,
}

/// Entry-major white/gold misses and each entry's touched-target mask for a chain key.
pub(super) struct KeyMisses {
    pub(super) white: Vec<f64>,
    pub(super) gold: Vec<f64>,
    pub(super) touched_targets: Vec<u32>,
}

/// Shared reduced entries and their identity in the dependent caches.
#[derive(Clone)]
pub(super) struct ReducedEntries {
    pub(super) entries: std::rc::Rc<Vec<usize>>,
    pub(super) set_id: usize,
}

/// Each entry's locked-card bit and the union required by every legal deck.
pub(super) struct LockedCards {
    pub(super) entry_bits: Vec<u8>,
    pub(super) all_bits: u8,
}

/// A pal without target hints, retained for legality and the reported deck.
pub(crate) struct PlainPal<'data> {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) card: &'data Card,
}

/// A monotone skyline coordinate: gold probability or probability of any hint at a target.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct HintCoordinate {
    pub(super) target: usize,
    pub(super) gold: bool,
}

/// Group-miss tables depend on the reduced entry set and chain-count code.
#[derive(PartialEq, Eq, Hash)]
pub(super) struct GroupMissKey {
    pub(super) set_id: usize,
    pub(super) code: u64,
}

/// Legal tables additionally depend on how many plain pals remain legal.
#[derive(PartialEq, Eq, Hash)]
struct LegalTableKey {
    set_id: usize,
    plain_pal_count: usize,
}

/// Dominance is specific to one group of one reduced entry set and chain code.
#[derive(PartialEq, Eq, Hash)]
pub(super) struct DominanceKey {
    pub(super) set_id: usize,
    pub(super) code: u64,
    pub(super) group: usize,
}
/// Every legal deck of one trainee, without the skyline's pruning (which compares product odds only). Used to make
/// the per-uma search exact: decks are enumerated by chain-kind key, like the skyline search, with an upper bound on
/// a partial deck's product value (each remaining slot of a kind group at that group's best miss chances per target,
/// ignoring the one-per-character rule), so only the decks that can reach a threshold are built.
pub(crate) struct DeckSpace<'data> {
    pub(crate) deck_job: &'data DeckJob,
    pub(super) target_groups: Vec<u32>,
    pub(super) skills: &'data SkillsRaw,
    pub(crate) cards: Vec<&'data Card>,
    // Parallel arrays indexed by entry, before card dominance: card index, kind, early end, variant.
    // Entry indices differ from card indices: one card can have several end-link/variant entries.
    pub(crate) card_indices: Vec<usize>,
    pub(crate) kind: Vec<ChainKind>,
    pub(super) end_links: Vec<u8>,
    pub(super) variants: Vec<u32>,
    pub(crate) sources: Vec<TargetSources>,
    pub(super) borrowed: Vec<bool>,
    // per entry, `cv_mask` of its source (the class caps)
    pub(super) class_masks: Vec<u64>,
    // pal characters with no relevant card: (id, name, card)
    pub(super) plain_pals: Vec<PlainPal<'data>>,
    pub(super) locked: Vec<usize>,
    pub(super) allowed_borrow: usize,
    // every chain kind of any entry plus fillers (2, 2): key codes are counts in base 7 over this list
    pub(super) kinds_all: Vec<ChainKind>,
    // All caches belong to this fixed job and target set. Their abbreviated keys omit only those fixed inputs.
    pub(super) key_miss_cache: std::cell::RefCell<crate::FxMap<u64, std::rc::Rc<KeyMisses>>>,
    pub(super) reduced_cache: std::cell::RefCell<crate::FxMap<Vec<crate::uma::UmaIndex>, ReducedEntries>>,
    pub(super) group_miss_cache: std::cell::RefCell<crate::FxMap<GroupMissKey, std::rc::Rc<GroupTable>>>,
    /// partial decks tested, over every search (the search's cost)
    pub(crate) nodes: std::cell::Cell<usize>,
    /// what `search` derives from a legal entry set alone, per (set id, plain pals allowed): built once, not per call
    legal_table_cache: std::cell::RefCell<crate::FxMap<LegalTableKey, std::rc::Rc<LegalTables>>>,
    /// every count vector over `kinds_all` (summing to at most deck_size), in enumeration order: the same for every search
    counts: std::cell::OnceCell<std::rc::Rc<Vec<Vec<usize>>>>,
    /// per entry, its locked card's bit (0 if none), and all locked bits together
    locked_card_bits: std::cell::OnceCell<LockedCards>,
    /// Per set id, the legal entries' miss rows per chain level.
    pub(super) state_row_cache: std::cell::RefCell<crate::FxMap<usize, std::rc::Rc<Vec<Option<Vec<MissFactors>>>>>>,
    /// the product search's dominance table (`dom_table`) per (set id, key code, group): functions of those alone
    pub(super) dominance_cache: std::cell::RefCell<crate::FxMap<DominanceKey, std::rc::Rc<Vec<Vec<Dominator>>>>>,
}
/// `search`'s per-entry-set tables: each entry's character bit (`too_many_characters` if more than 128),
/// the entries per chain kind and the pal (kind 0) entries, the locked cards each group can hold, the plain pals legal
pub(super) struct LegalTables {
    /// More than 128 characters cannot be represented by `character_bits`.
    pub(super) too_many_characters: bool,
    pub(super) character_bits: Vec<u128>,
    pub(super) by_kind: Vec<Vec<usize>>,
    pub(super) pal_entries: Vec<usize>,
    /// Locked-card bits per group: `by_kind` followed by `pal_entries`.
    pub(super) group_lock_bits: Vec<u8>,
    pub(super) plain_pal_count: usize,
}
/// A deck a search selected: its relevant entries, how many plain pals and filler cards fill it, and its hint chances
/// per uncertain target.
#[derive(Clone)]
pub(crate) struct SelectedDeck {
    /// Indices into this DeckSpace's parallel entry arrays, sorted before returning a search result.
    pub(crate) entries: Vec<usize>,
    /// How many plain pals (pal characters with no relevant card) the deck holds.
    pub(crate) plain_pals: usize,
    /// How many filler cards (any SR without target hints) complete it.
    pub(crate) filler_cards: usize,
    /// Per uncertain target, the deck's chance of a white hint when it gives no gold one.
    pub(crate) white_hint_chances: Vec<f64>,
    /// Per uncertain target, the deck's chance of a gold hint.
    pub(crate) gold_hint_chances: Vec<f64>,
}
/// Entry-major miss probabilities: entry * target_count + target, white and gold respectively.
#[derive(Clone, Copy)]
pub(crate) struct EntryMisses<'table> {
    pub(super) white: &'table [f64],
    pub(super) gold: &'table [f64],
    pub(super) target_count: usize,
}

impl<'data> DeckSpace<'data> {
    /// per-entry miss chances (white given no gold, gold), entry-major, under the chain odds of key `code`
    /// per entry and target the miss chances (white, gold) under the key, and per entry the targets it touches (bit t:
    /// a miss chance other than exactly 1.0)
    pub(super) fn key_miss(&self, code: u64) -> std::rc::Rc<KeyMisses> {
        if let Some(r) = self.key_miss_cache.borrow().get(&code) {
            return std::rc::Rc::clone(r);
        }
        let target_count = self.target_groups.len();
        let n = self.card_indices.len();
        let mut ks: Vec<ChainKind> = vec![];
        let mut c = code;
        for &kd in &self.kinds_all {
            for _ in 0..(c % 7) {
                ks.push(kd);
            }
            c /= 7;
        }
        let marg = chain_marginals(self.deck_job.chain_events(), &ks);
        let empty: Vec<f64> = vec![];
        let mut mw = vec![1.0f64; n * target_count];
        let mut mg = vec![1.0f64; n * target_count];
        for j in 0..n {
            self.sources[j].src_miss(
                marg.get(&self.kind[j]).unwrap_or(&empty),
                &mut mw[j * target_count..(j + 1) * target_count],
                &mut mg[j * target_count..(j + 1) * target_count],
            );
        }
        assert!(target_count <= 32, "{target_count} uncertain targets: at most 32 (MAX_UNCERTAIN)");
        let tm: Vec<u32> = (0..n)
            .map(|j| {
                (0..target_count)
                    .filter(|&t| mw[j * target_count + t] != 1.0 || mg[j * target_count + t] != 1.0)
                    .fold(0u32, |m, t| m | 1 << t)
            })
            .collect();
        let r = std::rc::Rc::new(KeyMisses { white: mw, gold: mg, touched_targets: tm });
        self.key_miss_cache.borrow_mut().insert(code, std::rc::Rc::clone(&r));
        r
    }
    /// Entries that matter for `trainees` (one per-uma trainee, or every structure member with a shared deck): legal
    /// ones (no
    /// card of any of them), with entries that are interchangeable for them merged. Targets every outfit of each has as
    /// a gold are constant in both the product and the exact value, so an entry is judged by its rewards for the other
    /// targets: entries of one character with the same such rewards and chain kind keep the first; entries with none
    /// (they only take a slot) keep, per chain kind, pal status and borrowing, deck_size of them, characters no
    /// rewarding entry uses first. Any deck has an equal-valued deck from these.
    pub(super) fn reduced(&self, model: &Model, trainees: &[crate::uma::UmaIndex], trainee_ids: &[u32]) -> Vec<usize> {
        let target_count = self.target_groups.len();
        let relt: Vec<bool> = (0..target_count)
            .map(|k| trainees.iter().any(|&u| model.opts[u].iter().any(|OutfitOption { coverage: st, .. }| st[k] < 2)))
            .collect();
        let project_group = |g: &Group| -> Group {
            g.iter()
                .map(|outcome| EventOutcome {
                    probability: outcome.probability,
                    rewards: outcome.rewards.iter().copied().filter(|reward| relt[reward.target]).collect::<Vec<_>>(),
                })
                .collect()
        };
        let proj = |x: &TargetSources| -> String {
            let tr: Vec<(usize, u64)> = x
                .training_hints
                .iter()
                .filter(|t| relt[t.target])
                .map(|t| (t.target, t.probability.to_bits()))
                .collect();
            let fx: Vec<Group> = x
                .fixed_outcomes
                .iter()
                .map(&project_group)
                .filter(|g| g.iter().any(|o| !o.rewards.is_empty()))
                .collect();
            let ch: Vec<ChainOutcome> = x
                .chain_outcomes
                .iter()
                .map(|group| ChainOutcome { link: group.link, outcomes: project_group(&group.outcomes) })
                .filter(|group| group.outcomes.iter().any(|outcome| !outcome.rewards.is_empty()))
                .collect();
            let bits = |g: &Group| -> Vec<(u64, Vec<(usize, u8)>)> {
                g.iter()
                    .map(|outcome| {
                        (
                            outcome.probability.to_bits(),
                            outcome.rewards.iter().map(|reward| (reward.target, reward.level)).collect(),
                        )
                    })
                    .collect()
            };
            format!(
                "{:?}|{:?}|{:?}",
                tr,
                fx.iter().map(bits).collect::<Vec<_>>(),
                ch.iter().map(|group| (group.link, bits(&group.outcomes))).collect::<Vec<_>>()
            )
        };
        let legal: Vec<usize> = (0..self.card_indices.len())
            .filter(|&j| {
                trainee_ids.iter().all(|&id| !self.cards[self.card_indices[j]].blocks(id, self.deck_job.checks))
            })
            .collect();
        let keys: Vec<String> = legal.iter().map(|&j| proj(&self.sources[j])).collect();
        let empty = proj(&TargetSources::default());
        let is_locked = |j: usize| self.locked.contains(&self.card_indices[j]);
        let mut keep: Vec<usize> = vec![];
        let mut seen: Vec<(u32, ChainKind, &String)> = vec![];
        let mut used_chara: Vec<u32> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] == empty && !is_locked(j) {
                continue;
            }
            let ch = self.cards[self.card_indices[j]].chara;
            if !is_locked(j) && seen.iter().any(|x| x.0 == ch && x.1 == self.kind[j] && x.2 == &keys[i]) {
                continue;
            }
            seen.push((ch, self.kind[j], &keys[i]));
            keep.push(j);
            used_chara.push(ch);
        }
        let mut cls: Vec<((ChainKind, bool, bool), Vec<usize>)> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] != empty || is_locked(j) {
                continue;
            }
            let c = (self.kind[j], self.cards[self.card_indices[j]].pal, self.borrowed[j]);
            match cls.iter_mut().find(|x| x.0 == c) {
                Some(x) => x.1.push(j),
                None => cls.push((c, vec![j])),
            }
        }
        let size = self.deck_job.deck_size;
        for (_, mem) in &cls {
            let mut pick: Vec<usize> = vec![];
            let mut chs: Vec<u32> = vec![];
            for pass in 0..2 {
                for &j in mem {
                    if pick.len() >= size {
                        break;
                    }
                    let ch = self.cards[self.card_indices[j]].chara;
                    if chs.contains(&ch) || pick.contains(&j) {
                        continue;
                    }
                    if pass == 0 && used_chara.contains(&ch) {
                        continue;
                    }
                    pick.push(j);
                    chs.push(ch);
                }
            }
            keep.extend(pick);
        }
        keep.sort_unstable();
        keep
    }
    /// (reduced entries, an id per distinct list for other caches): trainees whose lists are equal (a deck class) share
    /// the id, so what is cached by it (`gm_cached`, `legal_tabs`) is built once for them all
    pub(super) fn reduced_cached(
        &self,
        model: &Model,
        trainees: &[crate::uma::UmaIndex],
        trainee_ids: &[u32],
    ) -> ReducedEntries {
        if let Some(r) = self.reduced_cache.borrow().get(trainees) {
            return r.clone();
        }
        let list = self.reduced(model, trainees, trainee_ids);
        let same = self.reduced_cache.borrow().values().find(|v| *v.entries == list).cloned();
        let r = same.unwrap_or_else(|| {
            let id = self.reduced_cache.borrow().values().map(|v| v.set_id + 1).max().unwrap_or(0);
            ReducedEntries { entries: std::rc::Rc::new(list), set_id: id }
        });
        self.reduced_cache.borrow_mut().insert(trainees.to_vec(), r.clone());
        r
    }
    /// (`need`: bit i set for the groups the key's configurations use; the others are left empty)
    /// (a `GroupTable` of rows r = 0..=size, each the white then the gold products per target: `gm_w`, `gm_g`)
    pub(super) fn gm_cached(
        &self,
        key: GroupMissKey,
        groups: &[&Vec<usize>],
        needed_groups: u64,
        misses: EntryMisses<'_>,
    ) -> std::rc::Rc<GroupTable> {
        if let Some(r) = self.group_miss_cache.borrow().get(&key) {
            return std::rc::Rc::clone(r);
        }
        // Each target's lowest misses, as the first `deck_size` of the stable ascending sort (`top_asc`).
        let deck_size = self.deck_job.deck_size;
        let (mut a, mut b) = (Vec::with_capacity(deck_size), Vec::with_capacity(deck_size));
        let mut r = GroupTable::new(groups.len(), needed_groups, (deck_size + 1) * 2 * misses.target_count);
        for (gi, l) in groups.iter().enumerate() {
            if needed_groups >> gi & 1 == 0 {
                continue;
            }
            let pre = r.grp_mut(gi);
            for t in 0..misses.target_count {
                a.clear();
                b.clear();
                for &j in *l {
                    top_asc(&mut a, deck_size, misses.white[j * misses.target_count + t]);
                    top_asc(&mut b, deck_size, misses.gold[j * misses.target_count + t]);
                }
                for r in 1..=deck_size {
                    pre[r * 2 * misses.target_count + t] =
                        pre[(r - 1) * 2 * misses.target_count + t] * a.get(r - 1).copied().unwrap_or(1.0);
                    pre[(r * 2 + 1) * misses.target_count + t] =
                        pre[((r - 1) * 2 + 1) * misses.target_count + t] * b.get(r - 1).copied().unwrap_or(1.0);
                }
            }
        }
        let r = std::rc::Rc::new(r);
        self.group_miss_cache.borrow_mut().insert(key, std::rc::Rc::clone(&r));
        r
    }
    /// `search`'s tables for one legal entry set (reduced list `legal`, its id, the trainees' ids), built on first use
    pub(super) fn legal_tabs(&self, legal: &[usize], set_id: usize, trainee_ids: &[u32]) -> std::rc::Rc<LegalTables> {
        let plain_pal_count = self.legal_plain(trainee_ids).len();
        let key = LegalTableKey { set_id, plain_pal_count };
        if let Some(r) = self.legal_table_cache.borrow().get(&key) {
            return std::rc::Rc::clone(r);
        }
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &j in legal {
            let n = chara_ix.len();
            chara_ix.entry(self.cards[self.card_indices[j]].chara).or_insert(n);
        }
        let too_many_characters = chara_ix.len() > 128;
        let character_bits: Vec<u128> = if too_many_characters {
            vec![]
        } else {
            (0..self.card_indices.len())
                .map(|j| chara_ix.get(&self.cards[self.card_indices[j]].chara).map_or(0, |&c| 1u128 << c))
                .collect()
        };
        let nk = self.kinds_all.len();
        let by_kind: Vec<Vec<usize>> = (0..nk)
            .map(|i| {
                legal
                    .iter()
                    .copied()
                    .filter(|&j| self.kind[j] != ChainKind::NONE && self.kind[j] == self.kinds_all[i])
                    .collect()
            })
            .collect();
        let pal_entries: Vec<usize> = legal.iter().copied().filter(|&j| self.kind[j] == ChainKind::NONE).collect();
        // Locked cards each group (`by_kind`, then `pal_entries`) can hold.
        let lb = |j: usize| self.locked.iter().position(|&l| l == self.card_indices[j]).map_or(0u8, |b| 1u8 << b);
        let group_lock_bits: Vec<u8> = by_kind
            .iter()
            .chain(std::iter::once(&pal_entries))
            .map(|l| l.iter().fold(0u8, |m, &j| m | lb(j)))
            .collect();
        let r = std::rc::Rc::new(LegalTables {
            too_many_characters,
            character_bits,
            by_kind,
            pal_entries,
            group_lock_bits,
            plain_pal_count,
        });
        self.legal_table_cache.borrow_mut().insert(key, std::rc::Rc::clone(&r));
        r
    }
    /// every count vector over `kinds_all` (each entry the number of cards of that kind, summing to at most deck_size)
    pub(super) fn counts_all(&self) -> std::rc::Rc<Vec<Vec<usize>>> {
        std::rc::Rc::clone(self.counts.get_or_init(|| {
            let (nk, size) = (self.kinds_all.len(), self.deck_job.deck_size);
            let mut counts: Vec<Vec<usize>> = vec![vec![]];
            for _ in 0..nk {
                counts = counts
                    .into_iter()
                    .flat_map(|c| {
                        let used: usize = c.iter().sum();
                        (0..=size - used).map(move |x| {
                            let mut d = c.clone();
                            d.push(x);
                            d
                        })
                    })
                    .collect();
            }
            std::rc::Rc::new(counts)
        }))
    }
    /// per entry, its locked card's bit (0 if none); and every locked card's bit
    pub(super) fn lock_bits(&self) -> &LockedCards {
        self.locked_card_bits.get_or_init(|| {
            let lock_bits: Vec<u8> = (0..self.card_indices.len())
                .map(|j| self.locked.iter().position(|&l| l == self.card_indices[j]).map_or(0, |b| 1u8 << b))
                .collect();
            let all_locked: u8 = if self.locked.is_empty() { 0 } else { ((1u16 << self.locked.len()) - 1) as u8 };
            LockedCards { entry_bits: lock_bits, all_bits: all_locked }
        })
    }
    /// the class caps (`EXACT_CAPS`, `CAPS_TEST`'s in their place, or all at AUDIT_CAP under `cap_audit`)
    pub(crate) fn caps(&self) -> [f64; 4] {
        if self.deck_job.cap_audit { [AUDIT_CAP; 4] } else { self.deck_job.checks.caps_test.unwrap_or(EXACT_CAPS) }
    }
    /// most decks one band search may return: EXACT_CAP, ten times that under `cap_audit` (its bands are far larger)
    pub(crate) const fn band_cap(&self) -> usize {
        if self.deck_job.cap_audit { 10 * EXACT_CAP } else { EXACT_CAP }
    }
    /// the highest class cap (EXACT_RHO unless `cap_audit`)
    pub(crate) fn rho_top(&self) -> f64 {
        self.caps()[3]
    }
    /// a deck's structure class (`cap_class`)
    pub(crate) fn sel_class(&self, x: &SelectedDeck) -> usize {
        cap_class(x.entries.iter().fold(0u64, |m, &j| m | self.class_masks[j]))
    }
    /// a deck's cap under the ceiling rho: its class's cap, at most rho
    pub(crate) fn sel_cap(&self, x: &SelectedDeck, rho: f64) -> f64 {
        self.caps()[self.sel_class(x)].min(rho)
    }
    pub(crate) fn legal_plain(&self, trainee_ids: &[u32]) -> Vec<&PlainPal<'data>> {
        self.plain_pals
            .iter()
            .filter(|x| trainee_ids.iter().all(|&id| !x.card.blocks(id, self.deck_job.checks)))
            .collect()
    }
    pub(crate) fn pdeck(&self, x: &SelectedDeck, trainee_ids: &[u32]) -> PDeck {
        let source_builder = Sources { dj: self.deck_job, unc: &self.target_groups };
        let lp = self.legal_plain(trainee_ids);
        let ci: Vec<usize> = x.entries.iter().map(|&j| self.card_indices[j]).collect();
        let borrowed = x.entries.iter().copied().find(|&j| self.borrowed[j]).map(|j| self.card_indices[j]);
        let mut names: Vec<String> = x
            .entries
            .iter()
            .map(|&j| {
                let c = self.cards[self.card_indices[j]];
                let e = self.end_links[j];
                let mut n = c.name.clone();
                let lb = source_builder.label(c, (e > 0).then_some(e), self.variants[j], self.skills);
                if !lb.is_empty() {
                    n = format!("{n} ({lb})");
                }
                if e > 0 {
                    n = format!("{n} (end chain at link {e})");
                }
                if Some(self.card_indices[j]) == borrowed {
                    n = format!("{n} (borrowed)");
                }
                n
            })
            .collect();
        names.extend(lp.iter().take(x.plain_pals).map(|p| p.name.clone()));
        for _ in 0..x.filler_cards {
            names.push("(any SR without target hints)".into());
        }
        PDeck {
            names: names.into_iter().map(Name::from).collect(),
            ids: ci.iter().map(|&i| self.cards[i].id).collect(),
            ends: x.entries.iter().map(|&j| self.end_links[j]).collect(),
            vars: x.entries.iter().map(|&j| self.variants[j]).collect(),
            choices: x
                .entries
                .iter()
                .map(|&j| {
                    Name::from(source_builder.label(
                        self.cards[self.card_indices[j]],
                        (self.end_links[j] > 0).then_some(self.end_links[j]),
                        self.variants[j],
                        self.skills,
                    ))
                })
                .collect(),
            plain_ids: lp.iter().take(x.plain_pals).map(|p| p.id).collect(),
            pal_names: ci
                .iter()
                .filter(|&&i| self.cards[i].pal)
                .map(|&i| Name::from(self.cards[i].name.as_str()))
                .chain(lp.iter().take(x.plain_pals).map(|p| Name::from(p.name.as_str())))
                .collect(),
            pals: x.entries.iter().filter(|&&j| self.cards[self.card_indices[j]].pal).count() + x.plain_pals,
            borrowed: borrowed.map(|i| Name::from(self.cards[i].name.as_str())),
            dw: x.white_hint_chances.clone(),
            dg: x.gold_hint_chances.clone(),
        }
    }
}

// ---------- what the packed search can represent (checked once, where a job's entries are built: `deck_entries`) ----------
/// Most cards in a deck: a key code counts each chain kind's cards as one base-7 digit (`DeckSpace::search`,
/// `key_miss`), so at most six of a kind. The page's decks hold six.
pub(crate) const MAX_DECK: usize = 6;
/// Most chain kinds (the fillers' (2, 2) included, `kinds_all`): a key code's largest value, six cards of the last kind,
/// must fit a u64 (it would up to 22), and the count vectors over the kinds (`counts_all`, C(16 + 6, 6) = 74613 of them
/// here) bound the configuration count.
pub(super) const MAX_KINDS: usize = 16;
/// Most uncertain targets: a deck entry's touched targets are the bits of a u32 (`key_miss`, `run_deck`).
pub(super) const MAX_UNCERTAIN: usize = 32;
// (locked cards are the bits of a u8, `lock_bits` and `LegalTables::group_lock_bits`: at most MAX_DECK are accepted)
const _: () = assert!(
    6 * 7u128.pow(MAX_KINDS as u32 - 1) <= u64::MAX as u128 && MAX_DECK < 7 && MAX_DECK <= 8,
    "kind counts are packed as base-7 digits of a u64 (a deck below 7 cards), locked cards as the bits of a u8"
);
/// Every chain kind of the entries `kind` (pals, `ChainKind::NONE`, have none) plus the fillers' (2, 2), sorted: the
/// digits of a key code
pub(super) fn kinds_all(kind: &[ChainKind]) -> Vec<ChainKind> {
    let mut kinds: Vec<ChainKind> = kind.iter().copied().filter(|&k| k != ChainKind::NONE).collect();
    kinds.push(ChainKind::FILLER);
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

pub(crate) fn deck_space<'data>(
    card_data: &'data CardsRaw,
    deck_job: &'data DeckJob,
    skills: &'data SkillsRaw,
    target_groups: &[u32],
    entries: &Entries<'data>,
) -> DeckSpace<'data> {
    let is_owned = |id: u32| {
        deck_job.owned_cards.is_empty() || deck_job.owned_cards.contains(&id) || deck_job.lock_cards.contains(&id)
    };
    let card_of: HashMap<u32, &Card> = card_data.cards.iter().map(|c| (c.id, c)).collect();
    let kinds_all = kinds_all(&entries.kind);
    DeckSpace {
        deck_job,
        target_groups: target_groups.to_vec(),
        skills,
        cards: entries.cards.clone(),
        card_indices: entries.rel.clone(),
        kind: entries.kind.clone(),
        end_links: entries.endk.clone(),
        variants: entries.vark.clone(),
        sources: entries.srcs.clone(),
        borrowed: entries.rel.iter().map(|&r| !is_owned(entries.cards[r].id)).collect(),
        class_masks: entries.srcs.iter().map(TargetSources::cv_mask).collect(),
        plain_pals: entries
            .plain_ids
            .iter()
            .zip(&entries.plain_names)
            .map(|(&id, n)| PlainPal { id, name: n.clone(), card: card_of[&id] })
            .collect(),
        locked: entries.locked.clone(),
        allowed_borrow: usize::from(deck_job.borrow),
        kinds_all,
        key_miss_cache: std::cell::RefCell::default(),
        reduced_cache: std::cell::RefCell::default(),
        group_miss_cache: std::cell::RefCell::default(),
        nodes: std::cell::Cell::default(),
        legal_table_cache: std::cell::RefCell::default(),
        counts: std::cell::OnceCell::default(),
        locked_card_bits: std::cell::OnceCell::default(),
        state_row_cache: std::cell::RefCell::default(),
        dominance_cache: std::cell::RefCell::default(),
    }
}
/// The first k values of a stable ascending sort (`sort_by(|x, y| x.partial_cmp(y).unwrap())`) of the values pushed in
/// turn, kept in `buf` (at most k, in that order): a value equal to some already kept goes after them, where the stable
/// sort puts it, so the kept values are the sorted prefix's, value for value (0 and -0 included).
/// This equivalence assumes non-NaN inputs, as the reference sort panics on an unordered comparison.
#[inline]
pub(super) fn top_asc(buf: &mut Vec<f64>, k: usize, value: f64) {
    if buf.len() == k {
        // Keep the existing comparison: negating it also rejects an unordered value (NaN).
        if k == 0 || !(value < buf[k - 1]) {
            return;
        }
        buf.pop();
    }
    // Strict comparison places a tie after the existing values, including signed-zero ties.
    let position = buf.iter().position(|&kept| kept > value).unwrap_or(buf.len());
    buf.insert(position, value);
}
/// Per-group tables of one key in one flat buffer (`gm_cached`'s and the ratio tables of `search`): the groups in `need`
/// each own a block of `block_len` f64s (built from 1.0), in group order; callers must not read omitted groups.
pub(crate) struct GroupTable {
    /// Packed block index per group; u32::MAX marks a group omitted by `need`.
    slot: Vec<u32>,
    block_len: usize,
    data: Vec<f64>,
}
impl GroupTable {
    pub(super) fn new(ngroups: usize, need: u64, block_len: usize) -> Self {
        let mut stored_groups = 0u32;
        let slot: Vec<u32> = (0..ngroups)
            .map(|group| {
                if need >> group & 1 == 1 {
                    stored_groups += 1;
                    stored_groups - 1
                } else {
                    u32::MAX
                }
            })
            .collect();
        Self { slot, block_len, data: vec![1.0; stored_groups as usize * block_len] }
    }
    /// Group's packed block; the caller must have included the group in `need`.
    #[inline(always)]
    pub(super) fn grp(&self, group: usize) -> &[f64] {
        let slot = self.slot[group] as usize;
        &self.data[slot * self.block_len..(slot + 1) * self.block_len]
    }
    #[inline(always)]
    pub(super) fn grp_mut(&mut self, group: usize) -> &mut [f64] {
        let slot = self.slot[group] as usize;
        &mut self.data[slot * self.block_len..(slot + 1) * self.block_len]
    }
}
/// `gm_cached`'s row r of a group's block: the white products per target, then the gold ones
#[inline(always)]
pub(super) fn gm_w(b: &[f64], target_count: usize, r: usize) -> &[f64] {
    &b[r * 2 * target_count..(r * 2 + 1) * target_count]
}
#[inline(always)]
pub(super) fn gm_g(b: &[f64], target_count: usize, r: usize) -> &[f64] {
    &b[(r * 2 + 1) * target_count..(r + 1) * 2 * target_count]
}
/// `top_asc` for the stable descending sort (`sort_by(|a, b| b.partial_cmp(a).unwrap())`)
#[inline]
pub(super) fn top_desc(buf: &mut Vec<f64>, k: usize, value: f64) {
    if buf.len() == k {
        // Keep the existing comparison: negating it also rejects an unordered value (NaN).
        if k == 0 || !(value > buf[k - 1]) {
            return;
        }
        buf.pop();
    }
    // Strict comparison places a tie after the existing values, including signed-zero ties.
    let position = buf.iter().position(|&kept| kept < value).unwrap_or(buf.len());
    buf.insert(position, value);
}
