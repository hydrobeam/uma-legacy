//! The joint deck and loop search (`run_deck`): structures in bands, schedules, ceilings and the shared-deck path.
use super::{
    CHAIN_EVENTS, CHAIN_N, CHAIN_P, Card, CardsRaw, Data, DeckEval, DeckJob, Deserialize, Entries, HashMap, Instant,
    Kept, Kind, Model, Name, PDeck, SkillJob, SkillsRaw, Sources, Src, Value, chain_marginals, deck_entries,
    deck_space, dominates, json, k_filter, kfilter_order, kmax_top, run_multi, sig_of, src_miss, tie_rank,
};

/// Deck + loop search. The character rule (a trainee cannot run a card of her own character, nor a group card she is
/// a member of, `Card::blocks`) is part of which decks are legal in the exact search (`DeckSpace`): per trainee with
/// per-uma decks, for all four members with a shared deck.
pub(crate) fn run_deck(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    run_deck_relaxed(d, sk, cr, v)
}

pub(super) fn run_deck_relaxed(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match SkillJob::deserialize(v) {
        Ok(x) => x,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let dj: DeckJob = match DeckJob::deserialize(v) {
        Ok(x) => x,
        Err(e) => return json!({"error": e.to_string()}),
    };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let model = Model::new(d, sk, &sj);
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let pool_ids: Vec<u32> = d
        .umas
        .iter()
        .map(|u| u.id)
        .filter(|&id| {
            (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)
        })
        .collect();
    let mut ent = match deck_entries(cr, &dj, &unc, &pool_ids) {
        Ok(e) => e,
        Err(e) => return e,
    };
    // every entry, before card dominance drops any: the exact per-uma search enumerates decks from these
    let space = (!dj.deck_only).then(|| deck_space(cr, &dj, sk, &unc, &ent));
    let srcb = Sources { dj: &dj, unc: &unc };
    let n_card_dropped = drop_dominated_cards(&mut ent, &dj, nu);
    let coords = dominance_coords(&ent.srcs, nu);
    let en = match Enum::new(&ent, &dj, nu, &coords) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let t0 = Instant::now();
    let mut ct = EnumCounts::default();
    // the decks no other deck beats on every dominance coordinate (the exact search scores every deck it needs from
    // `DeckSpace`, so one skyline serves any `top_decks`)
    let front: Vec<DeckEval> = en.run_enum(1, None, &mut ct);
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!(
            "tests {} scans {} avg_lim {:.0} dom_in_scan {} ring {} swapped {} subtrees {} s1 {}",
            ct.st_tests, ct.st_full, 0.0, ct.st_dom, ct.st_ring, ct.n_swapped, ct.n_subtrees, ct.st_s1
        );
    }
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!("rel {} decks {} enum {:.2}s", en.nrel, ct.n_decks, t0.elapsed().as_secs_f64());
    }
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!("front {} pareto {:.2}s", front.len(), t0.elapsed().as_secs_f64());
    }
    if front.is_empty() {
        return json!({"error": "No deck can be built from the allowed cards."});
    }
    let sky = skyline(&front, &dj, coords.len());
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!("skyline {} of {}", sky.len(), front.len());
    }
    let n_front = front.len();
    let pd = detach(front, &sky, &ent, &srcb, sk);
    if dj.deck_only {
        return json!({"sky": pd});
    }
    let stats = json!({"decks_built": ct.n_decks, "decks_on_frontier": n_front, "relevant_cards": en.nrel, "cards_dropped": n_card_dropped,
                       "decks_swapped": ct.n_swapped, "plain_pals": ent.plain_names});
    run_multi(d, cr, &model, &sj, &dj, &pd, stats, space.as_ref().unwrap())
}

/// Whether the player owns a card (a locked card counts as owned)
fn is_owned(dj: &DeckJob, id: u32) -> bool {
    dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id)
}

/// Calls `out` on every multiset of at most `left` values from `vals[start..]` (as index-ordered lists)
fn multisets(vals: &[Kind], start: usize, left: usize, cur: &mut Vec<Kind>, out: &mut dyn FnMut(&[Kind])) {
    out(cur);
    if left == 0 {
        return;
    }
    for i in start..vals.len() {
        cur.push(vals[i]);
        multisets(vals, i, left - 1, cur, out);
        cur.pop();
    }
}

/// ---- card dominance (exact) ----
/// Card A can stand in for card B in any deck when both are pals or both have the same chain length (so the
/// deck's chain odds do not change), A is owned whenever B is, and A at its weakest still beats B at its
/// strongest on every target: A's miss chances with its chain links at their lowest possible odds (the most
/// crowded deck) are no higher than B's with every link firing, for both "no gold hint" and "no hint at all".
/// Swapping B for A then gives a deck at least as good on every dominance coordinate, with the same pal
/// count. The swap is blocked only if A is already in the deck or shares a character with one of the other
/// deck_size - 1 cards, so B is dropped when kept dominators with at least deck_size - 1 + top_decks
/// distinct characters exist: every deck with B then has top_decks distinct kept decks at least as good
/// (swap the deck's other dropped cards first, then B for each unblocked dominator).
/// Drops them from every entry column; how many it dropped.
fn drop_dominated_cards(ent: &mut Entries, dj: &DeckJob, nu: usize) -> usize {
    let Entries { cards, rel, kind, endk, vark, srcs, locked, .. } = ent;
    let mut n_card_dropped = 0usize;
    // pals and R cards both take no chain slot, so they can stand in for each other unless the job fixes the pal
    // count
    let pal_fixed = dj.pals.is_some();
    let nrel0 = rel.len();
    let (lo, hi) = miss_bounds(kind, srcs, dj.deck_size.saturating_sub(1), nu);
    let strength = |j: usize| (0..nu).map(|k| 2.0 - lo[j].0[k] - lo[j].1[k]).sum::<f64>();
    let mut order: Vec<usize> = (0..nrel0).collect();
    let tr = tie_rank(rel, cards);
    order.sort_by(|&a, &b| strength(b).partial_cmp(&strength(a)).unwrap().then(tr[a].cmp(&tr[b])));
    let need = dj.deck_size.saturating_sub(1) + kmax_top(dj);
    let mut keep = vec![true; nrel0];
    let mut done: Vec<usize> = vec![];
    for &b in &order {
        let cb = cards[rel[b]];
        if !locked.contains(&rel[b]) {
            let mut charas: Vec<u32> = vec![];
            for &a in &done {
                if !keep[a] {
                    continue;
                }
                let ca = cards[rel[a]];
                if kind[a] != kind[b] || (pal_fixed && ca.pal != cb.pal) {
                    continue;
                }
                if is_owned(dj, cb.id) && !is_owned(dj, ca.id) {
                    continue;
                }
                if !(0..nu).all(|k| hi[a].0[k] <= lo[b].0[k] && hi[a].1[k] <= lo[b].1[k]) {
                    continue;
                }
                if !charas.contains(&ca.chara) {
                    charas.push(ca.chara);
                }
                if charas.len() >= need {
                    break;
                }
            }
            if charas.len() >= need {
                keep[b] = false;
                n_card_dropped += 1;
            }
        }
        done.push(b);
    }
    if std::env::var("DECK_DEBUG").is_ok() {
        for j in 0..nrel0 {
            if !keep[j] {
                eprintln!("  drop {}", cards[rel[j]].name);
            }
        }
    }
    if std::env::var("DECK_DEBUG_PAIRS").is_ok() {
        debug_robust(cards, rel, kind, &keep, &lo, &hi, &tr, pal_fixed, nu);
    }
    // All entry columns use the same original-index mask and keep their relative order.
    retain_entries(rel, &keep);
    retain_entries(srcs, &keep);
    retain_entries(kind, &keep);
    retain_entries(endk, &keep);
    retain_entries(vark, &keep);
    n_card_dropped
}

/// Each entry's (miss gold, miss any) per target at its strongest (every link firing) and weakest (its links at their
/// lowest odds over any `others` other chain cards)
fn miss_bounds(kind: &[Kind], srcs: &[Src], others: usize, nu: usize) -> (MissTab, MissTab) {
    let nrel0 = kind.len();
    let mut kindset: Vec<Kind> = (0..nrel0).filter(|&j| kind[j] != (0, 0, 0)).map(|j| kind[j]).collect();
    kindset.push((2, 2, 0));
    kindset.sort_unstable();
    kindset.dedup();
    // lowest odds of each link over every possible set of other chain cards
    let needs_marg = |kd: Kind| (0..nrel0).any(|j| kind[j] != (0, 0, 0) && kind[j] == kd && !srcs[j].chains.is_empty());
    let mut min_marg: HashMap<Kind, Vec<f64>> = HashMap::new();
    for &kd in &kindset {
        if !needs_marg(kd) {
            continue;
        }
        let mut mins = vec![1.0f64; kd.1 as usize];
        multisets(&kindset, 0, others, &mut vec![], &mut |o| {
            let mut key = vec![kd];
            key.extend_from_slice(o);
            let m = chain_marginals(&key);
            for k in 0..kd.1 as usize {
                mins[k] = mins[k].min(m[&kd][k]);
            }
        });
        min_marg.insert(kd, mins);
    }
    // per card: (miss gold, miss any) per target, at its strongest (lo) and weakest (hi)
    let miss = |j: usize, strongest: bool| -> (Vec<f64>, Vec<f64>) {
        let stop = kind[j].1 as usize;
        let m: Vec<f64> = if strongest {
            vec![1.0; stop]
        } else {
            min_marg.get(&kind[j]).cloned().unwrap_or_else(|| vec![0.0; stop])
        };
        let mut mw = vec![1.0f64; nu];
        let mut mg = vec![1.0f64; nu];
        src_miss(&srcs[j], &m, &mut mw, &mut mg);
        let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
        (mg, mh)
    };
    let lo: Vec<(Vec<f64>, Vec<f64>)> = (0..nrel0).map(|j| miss(j, true)).collect();
    let hi: Vec<(Vec<f64>, Vec<f64>)> = (0..nrel0).map(|j| miss(j, false)).collect();
    (lo, hi)
}
/// per entry: (miss gold, miss any) per target
type MissTab = Vec<(Vec<f64>, Vec<f64>)>;

/// DECK_DEBUG_PAIRS: per kept card, its robust dominators among the kept ones (the same test as the drop, exact ties
/// by `tie_rank`)
fn debug_robust(
    cards: &[&Card],
    rel: &[usize],
    kind: &[Kind],
    keep: &[bool],
    lo: &MissTab,
    hi: &MissTab,
    tr: &[usize],
    pal_fixed: bool,
    nu: usize,
) {
    let nrel0 = rel.len();
    // Per kept card, robust dominators reported by DECK_DEBUG_PAIRS (same test as above).
    let kept_old: Vec<usize> = (0..nrel0).filter(|&j| keep[j]).collect();
    let robust: Vec<Vec<usize>> = kept_old
        .iter()
        .map(|&b| {
            let cb = cards[rel[b]];
            kept_old
                .iter()
                .enumerate()
                .filter(|&(_, &a)| {
                    if a == b {
                        return false;
                    }
                    let ca = cards[rel[a]];
                    if kind[a] != kind[b] || (pal_fixed && ca.pal != cb.pal) {
                        return false;
                    }
                    let le = (0..nu).all(|k| hi[a].0[k] <= lo[b].0[k] && hi[a].1[k] <= lo[b].1[k]);
                    let eq = (0..nu).all(|k| hi[a].0[k] == lo[b].0[k] && hi[a].1[k] == lo[b].1[k]);
                    le && (!eq || tr[a] < tr[b])
                })
                .map(|(ni, _)| ni)
                .collect()
        })
        .collect();
    for (bi, &b) in kept_old.iter().enumerate() {
        let names: Vec<String> = robust[bi].iter().map(|&a| cards[rel[kept_old[a]]].name.clone()).collect();
        if !names.is_empty() {
            eprintln!("ROBUST {} <= {:?}", cards[rel[b]].name, names);
        }
    }
}

/// ---- dominance coordinates ----
/// For a target the trainee's outfit does not give, the metric's landing chance is
///   p = W + W*g - W*(1 - q)*(1 - h)  (W = GEN_WHITE),   g = P(gold hint), h = P(any hint) = 1 - (1 - w)*(1 - g);
/// with the white on the outfit p = W + W*g, and with the gold it is constant. So the metric only
/// rises with g and h, and a deck at least as high in every (g, h) is never worse. (w, g) dominance implies
/// (g, h) dominance, so this prunes at least as much. Coordinates no relevant card can move are dropped,
/// and h == g when no card gives the white.
fn dominance_coords(srcs: &[Src], nu: usize) -> Vec<(usize, bool)> {
    let mut has_w = vec![false; nu];
    let mut has_g = vec![false; nu];
    for sr in srcs {
        for &(k, _) in &sr.train {
            has_w[k] = true;
        }
        for g in sr.fixed.iter().chain(sr.chains.iter().map(|x| &x.1)) {
            for (_, out) in g {
                for &(k, lvl) in out {
                    if lvl >= 2 {
                        has_g[k] = true;
                    } else {
                        has_w[k] = true;
                    }
                }
            }
        }
    }
    (0..nu)
        .flat_map(|k| {
            let mut v = vec![];
            if has_g[k] {
                v.push((k, true));
            }
            if has_w[k] {
                v.push((k, false));
            }
            v
        })
        .collect()
}

/// Per chain-kind key: each entry's miss chances (white, gold) under the key's chain odds, and for each entry B the
/// entries that beat it on every target under those odds (same kind, so swapping keeps the odds; per target no
/// higher chance of missing the gold and no higher chance of missing every hint; exact ties go by `tie_rank`), as
/// owned / borrowed character masks.
struct KeyData {
    mw: Vec<f64>,
    mg: Vec<f64>,
    d_own: Vec<u128>,
    d_bor: Vec<u128>,
}

/// Work counters of the enumeration (`decks_built`, `decks_swapped` and DECK_DEBUG's)
#[derive(Default)]
struct EnumCounts {
    n_decks: usize,
    n_swapped: usize,
    n_subtrees: usize,
    st_s1: usize,
    st_tests: usize,
    st_full: usize,
    st_dom: usize,
    st_ring: usize,
}

/// The decks one pick builds: their entries (`target`), plain pals and fillers
#[derive(Clone, Copy)]
struct Shape {
    target: usize,
    plain: usize,
    filler: usize,
}

/// The kq = 1 result and its deck identities (`run_enum`)
type S1<'set> = Option<&'set (Kept, crate::FxSet<Vec<u64>>)>;
/// The frontier being built, per group (one global group, (0, 0))
type Groups = crate::FxMap<(usize, usize), Kept>;

// ---- enumerate decks ----
// Decks are enumerated one chain-kind key at a time (the key fixes every entry's chain odds), so each deck is
// generated once, under its own key, with that key's exact dominance relation:
//  - Swap rule. If card A outside the deck beats deck card B on every target under the key's odds and can
//    legally replace it (same kind, so the key is unchanged; not a second entry of a character already in
//    the deck; within the borrow limit), the swapped deck is generated too and is at least as good. A deck
//    with top_decks such swaps is not needed: by induction over the swap order (ties go to the lower index,
//    so it has no cycles), each swapped deck is kept or itself has top_decks kept decks at least as good.
//    Distinct (B, A) pairs give distinct decks; counting A's distinct characters undercounts pairs, which
//    only skips fewer decks.
//  - Partial decks. While a deck is built, each entry still to be added blocks at most one more character,
//    so a partial deck where one card B already has (entries still to add) + top_decks unblocked legal
//    swaps can only grow into decks the swap rule skips; the whole branch is dropped.
//  - Dead entries. The same with nothing chosen yet: an entry whose legal dominators under the key span
//    deck_size - 1 + top_decks characters never appears.
// Owned cards only count borrowed dominators when the deck has room for a borrowed card.

/// The enumeration's tables over the entries left after card dominance: characters as bits (one deck holds one entry
/// per character), borrowed and locked flags, the chain kinds with their key places and entries, and the entries
/// without chain slots.
struct Enum<'env> {
    dj: &'env DeckJob,
    ent: &'env Entries<'env>,
    coords: &'env [(usize, bool)],
    nu: usize,
    nrel: usize,
    size: usize,
    allowed_borrow: usize,
    pal_fixed: bool,
    ebit: Vec<u128>,
    ebor: Vec<bool>,
    is_locked: Vec<bool>,
    // Chain-kind key of a deck = multiset of its chain entries' kinds plus fillers (2, 2), coded as counts in base 7.
    kinds_all: Vec<Kind>,
    kpos: Vec<u64>,
    ki22: usize,
    by_kind: Vec<Vec<usize>>,
    // entries without chain slots: pals, group cards and R cards
    pal_ents: Vec<usize>,
    tr2: Vec<usize>,
}

impl<'env> Enum<'env> {
    fn new(
        ent: &'env Entries<'env>,
        dj: &'env DeckJob,
        nu: usize,
        coords: &'env [(usize, bool)],
    ) -> Result<Self, Value> {
        let (cards, rel, kind) = (&ent.cards, &ent.rel, &ent.kind);
        let allowed_borrow = usize::from(dj.borrow);
        let nrel = rel.len();
        // characters as bits (one deck holds one entry per character)
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &r in rel {
            let n = chara_ix.len();
            chara_ix.entry(cards[r].chara).or_insert(n);
        }
        if chara_ix.len() > 128 {
            return Err(json!({"error": "too many distinct characters among relevant cards"}));
        }
        let ebit: Vec<u128> = rel.iter().map(|&r| 1u128 << chara_ix[&cards[r].chara]).collect();
        let ebor: Vec<bool> = rel.iter().map(|&r| !is_owned(dj, cards[r].id)).collect();
        let mut kinds_all: Vec<Kind> = (0..nrel).filter(|&j| kind[j] != (0, 0, 0)).map(|j| kind[j]).collect();
        kinds_all.push((2, 2, 0));
        kinds_all.sort_unstable();
        kinds_all.dedup();
        let kpos: Vec<u64> = (0..kinds_all.len()).map(|i| 7u64.pow(i as u32)).collect();
        let tr2 = tie_rank(rel, cards);
        assert!(nu <= 32, "too many uncertain targets");
        let is_locked: Vec<bool> = (0..nrel).map(|j| ent.locked.contains(&rel[j])).collect();
        let nk = kinds_all.len();
        let ki22 = kinds_all.iter().position(|&k| k == (2, 2, 0)).unwrap();
        let by_kind: Vec<Vec<usize>> =
            (0..nk).map(|i| (0..nrel).filter(|&j| kind[j] != (0, 0, 0) && kind[j] == kinds_all[i]).collect()).collect();
        let pal_ents: Vec<usize> = (0..nrel).filter(|&j| kind[j] == (0, 0, 0)).collect();
        Ok(Enum {
            dj,
            ent,
            coords,
            nu,
            nrel,
            size: dj.deck_size,
            allowed_borrow,
            pal_fixed: dj.pals.is_some(),
            ebit,
            ebor,
            is_locked,
            kinds_all,
            kpos,
            ki22,
            by_kind,
            pal_ents,
            tr2,
        })
    }

    /// The key data of a chain-kind key code (`KeyData`)
    fn make_key(&self, code: u64) -> KeyData {
        let (nrel, nu) = (self.nrel, self.nu);
        let (cards, rel, kind, srcs) = (&self.ent.cards, &self.ent.rel, &self.ent.kind, &self.ent.srcs);
        let mut ks: Vec<Kind> = vec![];
        let mut c = code;
        for &kd in &self.kinds_all {
            for _ in 0..(c % 7) {
                ks.push(kd);
            }
            c /= 7;
        }
        let marg = chain_marginals(&ks);
        let empty: Vec<f64> = vec![];
        let mut mw = vec![1.0f64; nrel * nu];
        let mut mg = vec![1.0f64; nrel * nu];
        for j in 0..nrel {
            src_miss(
                &srcs[j],
                marg.get(&kind[j]).unwrap_or(&empty),
                &mut mw[j * nu..(j + 1) * nu],
                &mut mg[j * nu..(j + 1) * nu],
            );
        }
        let mh: Vec<f64> = (0..nrel * nu).map(|i| mg[i] * mw[i]).collect();
        let mut d_own = vec![0u128; nrel];
        let mut d_bor = vec![0u128; nrel];
        for b in 0..nrel {
            for a in 0..nrel {
                // same kind keeps the chain odds; with a fixed pal count (`pals`), same pal status keeps it
                if a == b || kind[a] != kind[b] || (self.pal_fixed && cards[rel[a]].pal != cards[rel[b]].pal) {
                    continue;
                }
                let (ra, rb) = (a * nu..(a + 1) * nu, b * nu..(b + 1) * nu);
                let le = mg[ra.clone()].iter().zip(&mg[rb.clone()]).all(|(x, y)| x <= y)
                    && mh[ra.clone()].iter().zip(&mh[rb.clone()]).all(|(x, y)| x <= y);
                if !le {
                    continue;
                }
                let eq = mg[ra.clone()] == mg[rb.clone()] && mh[ra] == mh[rb];
                if eq && self.tr2[a] > self.tr2[b] {
                    continue;
                }
                if self.ebor[a] {
                    d_bor[b] |= self.ebit[a];
                } else {
                    d_own[b] |= self.ebit[a];
                }
            }
        }
        KeyData { mw, mg, d_own, d_bor }
    }

    /// a deck's identity from a selection of entries
    fn ident_sel(&self, sel: &[usize], plain: usize, filler: usize) -> Vec<u64> {
        let ident_of = |c: usize, e: u8, x: u32| ((c as u64 + 3) << 40) | (u64::from(x) << 8) | u64::from(e);
        let mut v: Vec<u64> = Vec::with_capacity(sel.len() + 2);
        v.extend(sel.iter().map(|&j| ident_of(self.ent.rel[j], self.ent.endk[j], self.ent.vark[j])));
        v.sort_unstable();
        v.push((1 << 40) + plain as u64);
        v.push((2 << 40) + filler as u64);
        v
    }

    /// One enumeration pass keeping the decks that fewer than `kq` others beat. With `s1` (the kq = 1 result and
    /// its deck identities) a deck is dropped at once when kq decks of that set, other than itself, beat it.
    fn run_enum(&self, kq: usize, s1: S1, ct: &mut EnumCounts) -> Vec<DeckEval> {
        let mut groups = Groups::default();
        let kmax = kq;
        let mut dv = vec![0.0f64; self.coords.len()];
        for cnt in &self.count_vectors() {
            self.enum_key(cnt, kmax, s1, &mut groups, &mut dv, ct);
        }
        let mut front: Vec<DeckEval> = vec![];
        let mut gkeys: Vec<(usize, usize)> = groups.keys().copied().collect();
        gkeys.sort_unstable();
        for pc in &gkeys {
            let list = groups.remove(pc).unwrap().into_list();
            // with kmax = 1 the online set is already an antichain (every insert drops what it dominates and is
            // rejected when dominated), so k_filter would keep every deck and only reorder: reproduce its order
            let kept = if kmax == 1 {
                kfilter_order(list, self.coords.len())
            } else {
                k_filter(list, kmax, self.coords.len()).into_list()
            };
            front.extend(kept);
        }
        front
    }

    /// every chain-kind count vector (fillers included in the (2, 2) count) with at most deck_size chain slots
    fn count_vectors(&self) -> Vec<Vec<usize>> {
        let size = self.size;
        let mut counts: Vec<Vec<usize>> = vec![vec![]];
        for _ in 0..self.kinds_all.len() {
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
        counts
    }

    /// The decks of one count vector: its key's dead entries left out, every split of the (2, 2) count into real
    /// entries and fillers and of the rest into pal-like entries and plain pals
    fn enum_key(&self, cnt: &[usize], kmax: usize, s1: S1, groups: &mut Groups, dv: &mut [f64], ct: &mut EnumCounts) {
        let (nk, ki22, size) = (self.kinds_all.len(), self.ki22, self.size);
        let (by_kind, ebor) = (&self.by_kind, &self.ebor);
        let csum: usize = cnt.iter().sum();
        // a key needs real entries of each non-filler kind
        if (0..nk).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) {
            return;
        }
        let code: u64 = cnt.iter().zip(&self.kpos).map(|(&c, &p)| c as u64 * p).sum();
        // (built per count vector and dropped after it: each one has its own code, so no key repeats)
        let kd: &KeyData = &self.make_key(code);
        let alive = |j: usize| {
            self.is_locked[j]
                || (kd.d_own[j] | if ebor[j] { kd.d_bor[j] } else { 0 }).count_ones() < (size - 1 + kmax) as u32
        };
        let alive_k: Vec<Vec<usize>> =
            (0..nk).map(|i| by_kind[i].iter().copied().filter(|&j| alive(j)).collect()).collect();
        let alive_p: Vec<usize> = self.pal_ents.iter().copied().filter(|&j| alive(j)).collect();
        for real22 in 0..=cnt[ki22].min(by_kind[ki22].len()) {
            let filler = cnt[ki22] - real22;
            if (0..nk).any(|i| (if i == ki22 { real22 } else { cnt[i] }) > alive_k[i].len()) {
                continue;
            }
            let rem = size - csum;
            for np in 0..=rem.min(alive_p.len()) {
                let plain = rem - np;
                if plain > self.ent.plain_chars.len() {
                    continue;
                }
                let target = csum - filler + np; // entries in the finished deck
                let mut gs: Vec<(Vec<usize>, usize)> = vec![];
                for i in 0..nk {
                    let c = if i == ki22 { real22 } else { cnt[i] };
                    if c > 0 {
                        gs.push((alive_k[i].clone(), c));
                    }
                }
                if np > 0 {
                    gs.push((alive_p.clone(), np));
                }
                self.enum_decks(kd, &gs, Shape { target, plain, filler }, kmax, s1, groups, dv, ct);
            }
        }
    }

    /// The decks picking `cnt` entries from each group `gs` in turn, with partial decks pruned by the swap rule
    fn enum_decks(
        &self,
        kd: &KeyData,
        gs: &[(Vec<usize>, usize)],
        sh: Shape,
        kmax: usize,
        s1: S1,
        groups: &mut Groups,
        dv: &mut [f64],
        ct: &mut EnumCounts,
    ) {
        let (ebit, ebor, is_locked) = (&self.ebit, &self.ebor, &self.is_locked);
        let mut n_pruned = 0usize;
        // (a card added later could be the deck's borrowed card, so before the deck is complete an owned
        // card only counts owned dominators)
        let mut prune = |sel: &[usize], chm: u128, _nb: usize| -> bool {
            let need = (sh.target - sel.len() + kmax) as u32;
            for &b in sel {
                if is_locked[b] {
                    continue;
                }
                let m = kd.d_own[b] | if ebor[b] { kd.d_bor[b] } else { 0 };
                if (m & !(chm & !ebit[b])).count_ones() >= need {
                    n_pruned += 1;
                    return true;
                }
            }
            false
        };
        let mut out = |sel: &[usize], chm: u128, nb: usize| self.emit(kd, sel, chm, nb, sh, kmax, s1, groups, dv, ct);
        let mut stack: Vec<usize> = Vec::with_capacity(self.size);
        let mut cut = 0usize;
        if gs.is_empty() {
            out(&stack, 0, 0);
        } else {
            // later[gi][p]: characters of entries after position p in group gi and of every later group
            let mut later: Vec<Vec<u128>> = vec![vec![]; gs.len()];
            let mut tail = 0u128;
            for gi in (0..gs.len()).rev() {
                let l = &gs[gi].0;
                let mut v = vec![0u128; l.len()];
                let mut m = tail;
                for p in (0..l.len()).rev() {
                    v[p] = m;
                    m |= ebit[l[p]];
                }
                tail = m;
                later[gi] = v;
            }
            // legal dominators whatever else the deck holds: owned ones, or any for a borrowed card
            let dm: Vec<u128> = (0..self.nrel)
                .map(|j| if is_locked[j] { 0 } else { kd.d_own[j] | if ebor[j] { kd.d_bor[j] } else { 0 } })
                .collect();
            let cx = Ctx { gs, later: &later, dm: &dm, ebit, ebor, allowed: self.allowed_borrow, kq: kmax as u32 };
            pick(&cx, 0, 0, gs[0].1, &mut stack, 0, 0, 0, &mut cut, &mut prune, &mut out);
        }
        ct.n_subtrees += n_pruned + cut;
    }

    /// A finished deck: kept on the frontier unless it breaks the job's pal count or locks, the swap rule skips it,
    /// or kq decks beat it
    fn emit(
        &self,
        kd: &KeyData,
        sel: &[usize],
        chm: u128,
        nb: usize,
        sh: Shape,
        kmax: usize,
        s1: S1,
        groups: &mut Groups,
        dv: &mut [f64],
        ct: &mut EnumCounts,
    ) {
        let (nu, coords) = (self.nu, self.coords);
        let Some(pals) = self.pal_count(sel, sh.plain) else { return };
        if self.swaps_skip(kd, sel, chm, nb, kmax) {
            ct.n_swapped += 1;
            return;
        }
        let mut mw = [1.0f64; 32];
        let mut mg = [1.0f64; 32];
        for &j in sel {
            let (w, g) = (&kd.mw[j * nu..(j + 1) * nu], &kd.mg[j * nu..(j + 1) * nu]);
            for t in 0..nu {
                mw[t] *= w[t];
                mg[t] *= g[t];
            }
        }
        for t in 0..nu {
            mw[t] = 1.0 - mw[t];
            mg[t] = 1.0 - mg[t];
        }
        let borrowed = sel.iter().copied().find(|&j| self.ebor[j]).map(|j| self.ent.rel[j]);
        ct.n_decks += 1;
        let mut s = 0.0;
        for (c, &(k, is_g)) in coords.iter().enumerate() {
            dv[c] = if is_g { mg[k] } else { 1.0 - (1.0 - mw[k]) * (1.0 - mg[k]) };
            s += dv[c];
        }
        self.offer(sel, sh, pals, borrowed, &mw, &mg, s, kmax, s1, groups, dv, ct);
    }

    /// The deck's pal count, None when it breaks the job's pal count or leaves out a locked card
    fn pal_count(&self, sel: &[usize], plain: usize) -> Option<usize> {
        let (cards, rel) = (&self.ent.cards, &self.ent.rel);
        let pals = sel.iter().filter(|&&j| cards[rel[j]].pal).count() + plain;
        if let Some(p) = self.dj.pals
            && p != pals
        {
            return None;
        }
        if !self.ent.locked.iter().all(|l| sel.iter().any(|&j| rel[j] == *l)) {
            return None;
        }
        Some(pals)
    }

    /// swap rule over all deck cards (pairs, see above): whether the deck has kmax legal swaps for better cards
    fn swaps_skip(&self, kd: &KeyData, sel: &[usize], chm: u128, nb: usize, kmax: usize) -> bool {
        let (ebit, ebor) = (&self.ebit, &self.ebor);
        let dmask = |b: usize, nb: usize| -> u128 {
            kd.d_own[b] | if nb - usize::from(ebor[b]) < self.allowed_borrow { kd.d_bor[b] } else { 0 }
        };
        let mut swaps = 0u32;
        for &b in sel {
            if self.is_locked[b] {
                continue;
            }
            swaps += (dmask(b, nb) & !(chm & !ebit[b])).count_ones();
            if swaps as usize >= kmax {
                break;
            }
        }
        swaps as usize >= kmax
    }

    /// A deck with odds (mw, mg) and dominance coordinates dv (sum s) offered to the frontier: dropped when kq decks
    /// beat it (the ring of recent winners, the kq = 1 set, the frontier), else inserted
    fn offer(
        &self,
        sel: &[usize],
        sh: Shape,
        pals: usize,
        borrowed: Option<usize>,
        mw: &[f64; 32],
        mg: &[f64; 32],
        s: f64,
        kmax: usize,
        s1: S1,
        groups: &mut Groups,
        dv: &mut [f64],
        ct: &mut EnumCounts,
    ) {
        let (nu, coords, rel) = (self.nu, self.coords, &self.ent.rel);
        let Shape { plain, filler, .. } = sh;
        // One global frontier: pals have already passed the job's count constraint. A deck
        // matched or beaten on every coordinate is no better for any loop.
        let nd = coords.len();
        let g = groups.entry((0, 0)).or_insert_with(|| Kept::new(nd));
        if kmax == 1 && g.has_last && dominates(&g.lastrow, dv) {
            return;
        }
        if kmax == 1 && nd > 0 && (0..g.rlen).any(|r| dominates(&g.recent[r * nd..(r + 1) * nd], dv)) {
            ct.st_ring += 1;
            return;
        }
        if kmax > 1 && g.rlen >= kmax && g.count_recent(dv) >= kmax {
            ct.st_ring += 1;
            return;
        }
        let xq = sig_of(dv, true);
        if let Some((s1k, ids)) = s1 {
            // kq decks of the set beat it: drop it, unless it is one of them (then it needs one more)
            if s1k.count_ro(dv, xq, s, kmax) >= kmax {
                let own = ids.contains(&self.ident_sel(sel, plain, filler));
                if !own || s1k.count_ro(dv, xq, s, kmax + 1) > kmax {
                    ct.st_s1 += 1;
                    return;
                }
            }
        }
        let n = g.count(dv, xq, s, kmax, &mut ct.st_tests);
        ct.st_full += 1;
        if n > 0 && (kmax > 1 || n >= kmax) && nd > 0 {
            g.remember_found();
        }
        if n >= kmax {
            ct.st_dom += 1;
            return;
        }
        // entry order, for display (distinct entries: the unstable sort is the sort), on the stack
        let mut sa = [0usize; 32];
        let so = &mut sa[..sel.len()];
        so.copy_from_slice(sel);
        so.sort_unstable();
        // (`dv` kept on the deck only where the compaction below re-reads it, kmax > 1)
        let x = DeckEval {
            pals,
            borrowed,
            cards: so.iter().map(|&j| rel[j]).collect(),
            ends: so.iter().map(|&j| self.ent.endk[j]).collect(),
            vars: so.iter().map(|&j| self.ent.vark[j]).collect(),
            plain,
            filler,
            dw: mw[..nu].to_vec(),
            dg: mg[..nu].to_vec(),
            dv: if kmax > 1 { dv.to_vec() } else { vec![] },
            s,
        };
        if kmax == 1 {
            g.remove_dominated_by(dv, sig_of(dv, false), s);
        }
        g.insert_dv(x, dv);
        if kmax > 1 && g.len() > g.cap {
            // compaction drops decks, so the remembered dominators are forgotten (Kept::new clears them)
            let list = std::mem::replace(g, Kept::new(nd)).into_list();
            *g = k_filter(list, kmax, nd);
            g.cap = (2 * g.len()).max(512);
        }
    }
}

// Pick `cnt` entries from each group in turn (one entry per character, at most `allowed` borrowed).
// Passed-over dominators: when entry j is added at position p of its group, a dominator of j whose character
// is not in the deck and belongs to no entry that can still be added (`later[gi][p]`) will never be in the
// finished deck, so it is a legal swap for j there. `acc` counts such guaranteed swaps along the branch; at
// top_decks the branch is cut (the swap rule would skip every deck in it).
struct Ctx<'tab> {
    gs: &'tab [(Vec<usize>, usize)],
    later: &'tab [Vec<u128>],
    dm: &'tab [u128],
    ebit: &'tab [u128],
    ebor: &'tab [bool],
    allowed: usize,
    kq: u32,
}
fn pick(
    cx: &Ctx,
    gi: usize,
    start: usize,
    left: usize,
    stack: &mut Vec<usize>,
    chm: u128,
    nb: usize,
    acc: u32,
    cut: &mut usize,
    prune: &mut dyn FnMut(&[usize], u128, usize) -> bool,
    out: &mut dyn FnMut(&[usize], u128, usize),
) {
    if left == 0 {
        if gi + 1 >= cx.gs.len() {
            out(stack, chm, nb);
        } else {
            pick(cx, gi + 1, 0, cx.gs[gi + 1].1, stack, chm, nb, acc, cut, prune, out);
        }
        return;
    }
    let list = &cx.gs[gi].0;
    for p in start..list.len() {
        if list.len() - p < left {
            break;
        }
        let j = list[p];
        if chm & cx.ebit[j] != 0 || (cx.ebor[j] && nb >= cx.allowed) {
            continue;
        }
        let (c2, n2) = (chm | cx.ebit[j], nb + usize::from(cx.ebor[j]));
        let acc2 = acc + (cx.dm[j] & !c2 & !cx.later[gi][p]).count_ones();
        if acc2 >= cx.kq {
            *cut += 1;
            continue;
        }
        stack.push(j);
        if !prune(stack, c2, n2) {
            pick(cx, gi, p + 1, left - 1, stack, c2, n2, acc2, cut, prune, out);
        }
        stack.pop();
    }
}

/// Decks that no other frontier deck (of any pal count) dominates. The metric rises with every dominance
/// coordinate, so for any loop the best value over the frontier is reached on this skyline; the ceilings
/// in run_multi (which only bound that best value) are built from it. Exact scoring obtains
/// any other needed legal decks from DeckSpace.
/// (with top_decks = 1 the frontier is already an antichain, so it is its own skyline)
fn skyline(front: &[DeckEval], dj: &DeckJob, nd: usize) -> Vec<usize> {
    if kmax_top(dj) == 1 {
        (0..front.len()).collect()
    } else {
        let tagged: Vec<DeckEval> = front
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut y = x.clone();
                y.cards = vec![i];
                y
            })
            .collect();
        let mut v: Vec<usize> = k_filter(tagged, 1, nd).into_list().into_iter().map(|y| y.cards[0]).collect();
        v.sort_unstable();
        v
    }
}

/// The skyline's decks detached from the search (names, ids and choice labels instead of indices): the frontier
/// consumed, the skyline's decks (ascending indices, `sky`) in order: each deck's lists move into its PDeck and the
/// rest is freed before `run_multi`
fn detach(front: Vec<DeckEval>, sky: &[usize], ent: &Entries, srcb: &Sources, sk: &SkillsRaw) -> Vec<PDeck> {
    let cards = &ent.cards;
    // per (card, end link, variant): its name with the choice and end-link notes, its choice label, and the name
    // marked borrowed (built at first need)
    let mut shown: crate::FxMap<(usize, u8, u32), (Name, Name, Option<Name>)> = crate::FxMap::default();
    // (and the shared strings of the card names, the plain pals and the filler)
    let mut cname: crate::FxMap<usize, Name> = crate::FxMap::default();
    let plain_rc: Vec<Name> = ent.plain_names.iter().map(|n| Name::from(n.as_str())).collect();
    let filler_rc: Name = Name::from("(any SR without target hints)");
    let mut skx = sky.iter().copied().peekable();
    front
        .into_iter()
        .enumerate()
        .filter(|&(i, _)| {
            if skx.peek() == Some(&i) {
                skx.next();
                true
            } else {
                false
            }
        })
        .map(|(_, x)| {
            let mut names: Vec<Name> = Vec::with_capacity(x.cards.len() + x.plain + x.filler);
            let mut choices: Vec<Name> = Vec::with_capacity(x.cards.len());
            for ((&i, &e), &v) in x.cards.iter().zip(&x.ends).zip(&x.vars) {
                let (n, lb, nb) = shown.entry((i, e, v)).or_insert_with(|| {
                    let mut n = cards[i].name.clone();
                    let lb = srcb.label(cards[i], (e > 0).then_some(e), v, sk);
                    if !lb.is_empty() {
                        n = format!("{n} ({lb})");
                    }
                    if e > 0 {
                        n = format!("{n} (end chain at link {e})");
                    }
                    (Name::from(n), Name::from(lb), None)
                });
                names.push(if Some(i) == x.borrowed {
                    std::rc::Rc::clone(nb.get_or_insert_with(|| Name::from(format!("{n} (borrowed)"))))
                } else {
                    std::rc::Rc::clone(n)
                });
                choices.push(std::rc::Rc::clone(lb));
            }
            names.extend(plain_rc.iter().take(x.plain).cloned());
            for _ in 0..x.filler {
                names.push(std::rc::Rc::clone(&filler_rc));
            }
            let mut cn =
                |i: usize| std::rc::Rc::clone(cname.entry(i).or_insert_with(|| Name::from(cards[i].name.as_str())));
            PDeck {
                names,
                ids: x.cards.iter().map(|&i| cards[i].id).collect(),
                choices,
                plain_ids: ent.plain_ids.iter().take(x.plain).copied().collect(),
                pal_names: x
                    .cards
                    .iter()
                    .filter(|&&i| cards[i].pal)
                    .map(|&i| cn(i))
                    .chain(plain_rc.iter().take(x.plain).cloned())
                    .collect(),
                pals: x.pals,
                borrowed: x.borrowed.map(&mut cn),
                ends: x.ends,
                vars: x.vars,
                dw: x.dw,
                dg: x.dg,
            }
        })
        .collect()
}

/// Apply an original-entry mask to one column of the parallel entry arrays.
/// Each column starts with the mask's length; Vec::retain preserves its surviving entries' order.
fn retain_entries<T>(entries: &mut Vec<T>, keep: &[bool]) {
    let mut original_index = 0;
    entries.retain(|_| {
        let retain = keep[original_index];
        original_index += 1;
        retain
    });
}
