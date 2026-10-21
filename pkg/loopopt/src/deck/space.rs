//! The space of legal decks (`DeckSpace`): entries by chain-kind key, legality tables and caps.
use super::{
    AUDIT_CAP, Card, CardsRaw, DeckJob, EXACT_CAP, EXACT_CAPS, Entries, Group, HashMap, Kind, Model, Name, PDeck,
    SbCfg, SkillsRaw, Sources, Src, cap_class, chain_marginals, cv_mask, src_miss, test_caps,
};

thread_local! { pub(super) static UCHK: std::cell::Cell<(usize, f64, usize)> = const { std::cell::Cell::new((0, 0.0, 0)) }; pub(super) static SPT: std::cell::Cell<(f64, f64, usize, usize)> = const { std::cell::Cell::new((0.0, 0.0, 0, 0)) }; }
/// Every legal deck of one trainee, without the skyline's pruning (which compares product odds only). Used to make
/// the per-uma search exact: decks are enumerated by chain-kind key, like the skyline search, with an upper bound on
/// a partial deck's product value (each remaining slot of a kind group at that group's best miss chances per target,
/// ignoring the one-per-character rule), so only the decks that can reach a threshold are built.
pub(super) struct DeckSpace<'data> {
    pub(super) dj: &'data DeckJob,
    pub(super) unc: Vec<u32>,
    pub(super) sk: &'data SkillsRaw,
    pub(super) cards: Vec<&'data Card>,
    // Parallel arrays indexed by entry, before card dominance: card index, kind, early end, variant.
    // Entry indices differ from card indices: one card can have several end-link/variant entries.
    pub(super) rel: Vec<usize>,
    pub(super) kind: Vec<Kind>,
    pub(super) endk: Vec<u8>,
    pub(super) vark: Vec<u32>,
    pub(super) srcs: Vec<Src>,
    pub(super) ebor: Vec<bool>,
    // per entry, `cv_mask` of its source (the class caps)
    pub(super) cvm: Vec<u64>,
    // pal characters with no relevant card: (id, name, card)
    pub(super) plain: Vec<(u32, String, &'data Card)>,
    pub(super) locked: Vec<usize>,
    pub(super) allowed_borrow: usize,
    // every chain kind of any entry plus fillers (2, 2): key codes are counts in base 7 over this list
    pub(super) kinds_all: Vec<Kind>,
    // All caches belong to this fixed job and target set. Their abbreviated keys omit only those fixed inputs.
    pub(super) keys: std::cell::RefCell<crate::FxMap<u64, std::rc::Rc<(Vec<f64>, Vec<f64>, Vec<u32>)>>>,
    pub(super) red: std::cell::RefCell<crate::FxMap<Vec<usize>, (std::rc::Rc<Vec<usize>>, usize)>>,
    pub(super) gmc: std::cell::RefCell<crate::FxMap<(usize, u64), std::rc::Rc<GTab>>>,
    /// partial decks tested, over every search (the search's cost)
    pub(super) nodes: std::cell::Cell<usize>,
    /// what `search` derives from a legal entry set alone, per (set id, plain pals allowed): built once, not per call
    legt: std::cell::RefCell<crate::FxMap<(usize, usize), std::rc::Rc<LegalTabs>>>,
    /// every count vector over `kinds_all` (summing to at most deck_size), in enumeration order: the same for every search
    counts: std::cell::OnceCell<std::rc::Rc<Vec<Vec<usize>>>>,
    /// per entry, its locked card's bit (0 if none), and all locked bits together
    lockbits: std::cell::OnceCell<(Vec<u8>, u8)>,
    /// the state-conditioned bound's tables (`SbCfg`) per (set id, key code, real (2, 2) entries, kind-0 entries), and
    /// per set id the legal entries' miss rows per chain level: functions of the entry set and configuration alone
    pub(super) sbcfg: std::cell::RefCell<crate::FxMap<(usize, u64, usize, usize), std::rc::Rc<SbCfg>>>,
    pub(super) sbrows: std::cell::RefCell<crate::FxMap<usize, std::rc::Rc<Vec<Option<Vec<(f64, f64)>>>>>>,
    /// the product search's dominance table (`dom_table`) per (set id, key code, group): functions of those alone
    pub(super) domc: std::cell::RefCell<crate::FxMap<(usize, u64, usize), std::rc::Rc<Vec<Vec<(u32, u32)>>>>>,
}
/// `search`'s per-entry-set tables: the character bit of each entry (`over`: more than 128 characters),
/// the entries per chain kind and the pal (kind 0) entries, the locked cards each group can hold, the plain pals legal
pub(super) struct LegalTabs {
    /// More than 128 characters cannot be represented by `ebit`.
    pub(super) over: bool,
    pub(super) ebit: Vec<u128>,
    pub(super) by_kind: Vec<Vec<usize>>,
    pub(super) pal_ents: Vec<usize>,
    /// Locked-card bits per group: `by_kind` followed by `pal_ents`.
    pub(super) glk: Vec<u8>,
    pub(super) nplain: usize,
}
#[derive(Clone)]
pub(super) struct Sel {
    /// Indices into this DeckSpace's parallel entry arrays, sorted before returning a search result.
    pub(super) ents: Vec<usize>,
    pub(super) plain: usize,
    pub(super) filler: usize,
    pub(super) dw: Vec<f64>,
    pub(super) dg: Vec<f64>,
}
impl<'data> DeckSpace<'data> {
    /// per-entry miss chances (white given no gold, gold), entry-major, under the chain odds of key `code`
    /// per entry and target the miss chances (white, gold) under the key, and per entry the targets it touches (bit t:
    /// a miss chance other than exactly 1.0)
    pub(super) fn key_miss(&self, code: u64) -> std::rc::Rc<(Vec<f64>, Vec<f64>, Vec<u32>)> {
        if let Some(r) = self.keys.borrow().get(&code) {
            return std::rc::Rc::clone(r);
        }
        let nu = self.unc.len();
        let n = self.rel.len();
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
        let mut mw = vec![1.0f64; n * nu];
        let mut mg = vec![1.0f64; n * nu];
        for j in 0..n {
            src_miss(
                &self.srcs[j],
                marg.get(&self.kind[j]).unwrap_or(&empty),
                &mut mw[j * nu..(j + 1) * nu],
                &mut mg[j * nu..(j + 1) * nu],
            );
        }
        assert!(nu <= 32, "{nu} uncertain targets: at most 32 (MAX_UNCERTAIN)");
        let tm: Vec<u32> = (0..n)
            .map(|j| (0..nu).filter(|&t| mw[j * nu + t] != 1.0 || mg[j * nu + t] != 1.0).fold(0u32, |m, t| m | 1 << t))
            .collect();
        let r = std::rc::Rc::new((mw, mg, tm));
        self.keys.borrow_mut().insert(code, std::rc::Rc::clone(&r));
        r
    }
    /// Entries that matter for trainees `us` (one per-uma trainee, or a loop's four with a shared deck): legal ones (no
    /// card of any of them), with entries that are interchangeable for them merged. Targets every outfit of each has as
    /// a gold are constant in both the product and the exact value, so an entry is judged by its rewards for the other
    /// targets: entries of one character with the same such rewards and chain kind keep the first; entries with none
    /// (they only take a slot) keep, per chain kind, pal status and borrowing, deck_size of them, characters no
    /// rewarding entry uses first. Any deck has an equal-valued deck from these.
    pub(super) fn reduced(&self, model: &Model, us: &[usize], ids: &[u32]) -> Vec<usize> {
        let nu = self.unc.len();
        let relt: Vec<bool> =
            (0..nu).map(|k| us.iter().any(|&u| model.opts[u].iter().any(|(st, _)| st[k] < 2))).collect();
        let pg = |g: &Group| -> Group {
            g.iter().map(|(p, o)| (*p, o.iter().copied().filter(|x| relt[x.0]).collect::<Vec<_>>())).collect()
        };
        let proj = |x: &Src| -> String {
            let tr: Vec<(usize, u64)> = x.train.iter().filter(|t| relt[t.0]).map(|t| (t.0, t.1.to_bits())).collect();
            let fx: Vec<Group> = x.fixed.iter().map(&pg).filter(|g| g.iter().any(|o| !o.1.is_empty())).collect();
            let ch: Vec<(u8, Group)> =
                x.chains.iter().map(|(l, g)| (*l, pg(g))).filter(|(_, g)| g.iter().any(|o| !o.1.is_empty())).collect();
            let bits = |g: &Group| -> Vec<(u64, Vec<(usize, u8)>)> {
                g.iter().map(|(p, o)| (p.to_bits(), o.clone())).collect()
            };
            format!(
                "{:?}|{:?}|{:?}",
                tr,
                fx.iter().map(bits).collect::<Vec<_>>(),
                ch.iter().map(|(l, g)| (*l, bits(g))).collect::<Vec<_>>()
            )
        };
        let legal: Vec<usize> =
            (0..self.rel.len()).filter(|&j| ids.iter().all(|&id| !self.cards[self.rel[j]].blocks(id))).collect();
        let keys: Vec<String> = legal.iter().map(|&j| proj(&self.srcs[j])).collect();
        let empty = proj(&Src::default());
        let is_locked = |j: usize| self.locked.contains(&self.rel[j]);
        let mut keep: Vec<usize> = vec![];
        let mut seen: Vec<(u32, Kind, &String)> = vec![];
        let mut used_chara: Vec<u32> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] == empty && !is_locked(j) {
                continue;
            }
            let ch = self.cards[self.rel[j]].chara;
            if !is_locked(j) && seen.iter().any(|x| x.0 == ch && x.1 == self.kind[j] && x.2 == &keys[i]) {
                continue;
            }
            seen.push((ch, self.kind[j], &keys[i]));
            keep.push(j);
            used_chara.push(ch);
        }
        let mut cls: Vec<((Kind, bool, bool), Vec<usize>)> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] != empty || is_locked(j) {
                continue;
            }
            let c = (self.kind[j], self.cards[self.rel[j]].pal, self.ebor[j]);
            match cls.iter_mut().find(|x| x.0 == c) {
                Some(x) => x.1.push(j),
                None => cls.push((c, vec![j])),
            }
        }
        let size = self.dj.deck_size;
        for (_, mem) in &cls {
            let mut pick: Vec<usize> = vec![];
            let mut chs: Vec<u32> = vec![];
            for pass in 0..2 {
                for &j in mem {
                    if pick.len() >= size {
                        break;
                    }
                    let ch = self.cards[self.rel[j]].chara;
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
    pub(super) fn reduced_cached(&self, model: &Model, us: &[usize], ids: &[u32]) -> (std::rc::Rc<Vec<usize>>, usize) {
        if let Some(r) = self.red.borrow().get(us) {
            return r.clone();
        }
        let list = self.reduced(model, us, ids);
        let same = self.red.borrow().values().find(|v| *v.0 == list).cloned();
        let r = same.unwrap_or_else(|| {
            let id = self.red.borrow().values().map(|v| v.1 + 1).max().unwrap_or(0);
            (std::rc::Rc::new(list), id)
        });
        self.red.borrow_mut().insert(us.to_vec(), r.clone());
        r
    }
    /// (`need`: bit i set for the groups the key's configurations use; the others are left empty)
    /// (a `GTab` of rows r = 0..=size, each the white then the gold products per target: `gm_w`, `gm_g`)
    pub(super) fn gm_cached(
        &self,
        u: usize,
        code: u64,
        groups: &[&Vec<usize>],
        need: u64,
        mw: &[f64],
        mg: &[f64],
        nu: usize,
        size: usize,
    ) -> std::rc::Rc<GTab> {
        if let Some(r) = self.gmc.borrow().get(&(u, code)) {
            return std::rc::Rc::clone(r);
        }
        // (each target's `size` lowest, as the first `size` of the stable ascending sort: `top_asc`)
        let (mut a, mut b) = (Vec::with_capacity(size), Vec::with_capacity(size));
        let mut r = GTab::new(groups.len(), need, (size + 1) * 2 * nu);
        for (gi, l) in groups.iter().enumerate() {
            if need >> gi & 1 == 0 {
                continue;
            }
            let pre = r.grp_mut(gi);
            for t in 0..nu {
                a.clear();
                b.clear();
                for &j in *l {
                    top_asc(&mut a, size, mw[j * nu + t]);
                    top_asc(&mut b, size, mg[j * nu + t]);
                }
                for r in 1..=size {
                    pre[r * 2 * nu + t] = pre[(r - 1) * 2 * nu + t] * a.get(r - 1).copied().unwrap_or(1.0);
                    pre[(r * 2 + 1) * nu + t] = pre[((r - 1) * 2 + 1) * nu + t] * b.get(r - 1).copied().unwrap_or(1.0);
                }
            }
        }
        let r = std::rc::Rc::new(r);
        self.gmc.borrow_mut().insert((u, code), std::rc::Rc::clone(&r));
        r
    }
    /// `search`'s tables for one legal entry set (reduced list `legal`, its id, the trainees' ids), built on first use
    pub(super) fn legal_tabs(&self, legal: &[usize], set_id: usize, ids: &[u32]) -> std::rc::Rc<LegalTabs> {
        let nplain = self.legal_plain(ids).len();
        let key = (set_id, nplain);
        if let Some(r) = self.legt.borrow().get(&key) {
            return std::rc::Rc::clone(r);
        }
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &j in legal {
            let n = chara_ix.len();
            chara_ix.entry(self.cards[self.rel[j]].chara).or_insert(n);
        }
        let over = chara_ix.len() > 128;
        let ebit: Vec<u128> = if over {
            vec![]
        } else {
            (0..self.rel.len())
                .map(|j| chara_ix.get(&self.cards[self.rel[j]].chara).map_or(0, |&c| 1u128 << c))
                .collect()
        };
        let nk = self.kinds_all.len();
        let by_kind: Vec<Vec<usize>> = (0..nk)
            .map(|i| {
                legal
                    .iter()
                    .copied()
                    .filter(|&j| self.kind[j] != (0, 0, 0) && self.kind[j] == self.kinds_all[i])
                    .collect()
            })
            .collect();
        let pal_ents: Vec<usize> = legal.iter().copied().filter(|&j| self.kind[j] == (0, 0, 0)).collect();
        // locked cards each group (by_kind, then pal_ents) can hold
        let lb = |j: usize| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0u8, |b| 1u8 << b);
        let glk: Vec<u8> =
            by_kind.iter().chain(std::iter::once(&pal_ents)).map(|l| l.iter().fold(0u8, |m, &j| m | lb(j))).collect();
        let r = std::rc::Rc::new(LegalTabs { over, ebit, by_kind, pal_ents, glk, nplain });
        self.legt.borrow_mut().insert(key, std::rc::Rc::clone(&r));
        r
    }
    /// every count vector over `kinds_all` (each entry the number of cards of that kind, summing to at most deck_size)
    pub(super) fn counts_all(&self) -> std::rc::Rc<Vec<Vec<usize>>> {
        std::rc::Rc::clone(self.counts.get_or_init(|| {
            let (nk, size) = (self.kinds_all.len(), self.dj.deck_size);
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
    pub(super) fn lock_bits(&self) -> &(Vec<u8>, u8) {
        self.lockbits.get_or_init(|| {
            let lbit: Vec<u8> = (0..self.rel.len())
                .map(|j| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0, |b| 1u8 << b))
                .collect();
            let all_locked: u8 = if self.locked.is_empty() { 0 } else { ((1u16 << self.locked.len()) - 1) as u8 };
            (lbit, all_locked)
        })
    }
    /// the class caps (`EXACT_CAPS`, or all at AUDIT_CAP under `cap_audit`)
    pub(crate) fn caps(&self) -> [f64; 4] {
        if self.dj.cap_audit { [AUDIT_CAP; 4] } else { test_caps().unwrap_or(EXACT_CAPS) }
    }
    /// most decks one band search may return: EXACT_CAP, ten times that under `cap_audit` (its bands are far larger)
    pub(crate) const fn band_cap(&self) -> usize {
        if self.dj.cap_audit { 10 * EXACT_CAP } else { EXACT_CAP }
    }
    /// the highest class cap (EXACT_RHO unless `cap_audit`)
    pub(crate) fn rho_top(&self) -> f64 {
        self.caps()[3]
    }
    /// a deck's structure class (`cap_class`)
    pub(crate) fn sel_class(&self, x: &Sel) -> usize {
        cap_class(x.ents.iter().fold(0u64, |m, &j| m | self.cvm[j]))
    }
    /// a deck's cap under the ceiling rho: its class's cap, at most rho
    pub(crate) fn sel_cap(&self, x: &Sel, rho: f64) -> f64 {
        self.caps()[self.sel_class(x)].min(rho)
    }
    pub(super) fn legal_plain(&self, ids: &[u32]) -> Vec<&(u32, String, &'data Card)> {
        self.plain.iter().filter(|x| ids.iter().all(|&id| !x.2.blocks(id))).collect()
    }
    pub(super) fn pdeck(&self, x: &Sel, ids: &[u32]) -> PDeck {
        let srcb = Sources { dj: self.dj, unc: &self.unc };
        let lp = self.legal_plain(ids);
        let ci: Vec<usize> = x.ents.iter().map(|&j| self.rel[j]).collect();
        let borrowed = x.ents.iter().copied().find(|&j| self.ebor[j]).map(|j| self.rel[j]);
        let mut names: Vec<String> = x
            .ents
            .iter()
            .map(|&j| {
                let c = self.cards[self.rel[j]];
                let e = self.endk[j];
                let mut n = c.name.clone();
                let lb = srcb.label(c, (e > 0).then_some(e), self.vark[j], self.sk);
                if !lb.is_empty() {
                    n = format!("{n} ({lb})");
                }
                if e > 0 {
                    n = format!("{n} (end chain at link {e})");
                }
                if Some(self.rel[j]) == borrowed {
                    n = format!("{n} (borrowed)");
                }
                n
            })
            .collect();
        names.extend(lp.iter().take(x.plain).map(|p| p.1.clone()));
        for _ in 0..x.filler {
            names.push("(any SR without target hints)".into());
        }
        PDeck {
            names: names.into_iter().map(Name::from).collect(),
            ids: ci.iter().map(|&i| self.cards[i].id).collect(),
            ends: x.ents.iter().map(|&j| self.endk[j]).collect(),
            vars: x.ents.iter().map(|&j| self.vark[j]).collect(),
            choices: x
                .ents
                .iter()
                .map(|&j| {
                    Name::from(srcb.label(
                        self.cards[self.rel[j]],
                        (self.endk[j] > 0).then_some(self.endk[j]),
                        self.vark[j],
                        self.sk,
                    ))
                })
                .collect(),
            plain_ids: lp.iter().take(x.plain).map(|p| p.0).collect(),
            pal_names: ci
                .iter()
                .filter(|&&i| self.cards[i].pal)
                .map(|&i| Name::from(self.cards[i].name.as_str()))
                .chain(lp.iter().take(x.plain).map(|p| Name::from(p.1.as_str())))
                .collect(),
            pals: x.ents.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + x.plain,
            borrowed: borrowed.map(|i| Name::from(self.cards[i].name.as_str())),
            dw: x.dw.clone(),
            dg: x.dg.clone(),
        }
    }
}

// ---------- what the packed search can represent (checked once, where a job's entries are built: `deck_entries`) ----------
/// Most cards in a deck: a key code counts each chain kind's cards as one base-7 digit (`DeckSpace::search`,
/// `key_miss`), so at most six of a kind. The page's decks hold six.
pub(super) const MAX_DECK: usize = 6;
/// Most chain kinds (the fillers' (2, 2) included, `kinds_all`): a key code's largest value, six cards of the last kind,
/// must fit a u64 (it would up to 22), and the count vectors over the kinds (`counts_all`, C(16 + 6, 6) = 74613 of them
/// here) stay a few MB. The exports give at most eight.
pub(super) const MAX_KINDS: usize = 16;
/// Most uncertain targets: a deck entry's touched targets are the bits of a u32 (`key_miss`, `run_deck`).
pub(super) const MAX_UNCERTAIN: usize = 32;
// (locked cards are the bits of a u8, `lock_bits` and `LegalTabs::glk`: no more than MAX_DECK are accepted)
const _: () = assert!(
    6 * 7u128.pow(MAX_KINDS as u32 - 1) <= u64::MAX as u128 && MAX_DECK < 7 && MAX_DECK <= 8,
    "kind counts are packed as base-7 digits of a u64 (a deck below 7 cards), locked cards as the bits of a u8"
);
/// Every chain kind of the entries `kind` (pals, kind (0, 0, 0), have none) plus the fillers' (2, 2), sorted: the digits
/// of a key code
pub(super) fn kinds_all(kind: &[Kind]) -> Vec<Kind> {
    let mut kinds: Vec<Kind> = kind.iter().copied().filter(|&k| k != (0, 0, 0)).collect();
    kinds.push((2, 2, 0));
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

pub(super) fn deck_space<'data>(
    cr: &'data CardsRaw,
    dj: &'data DeckJob,
    sk: &'data SkillsRaw,
    unc: &[u32],
    e: &Entries<'data>,
) -> DeckSpace<'data> {
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let kinds_all = kinds_all(&e.kind);
    DeckSpace {
        dj,
        unc: unc.to_vec(),
        sk,
        cards: e.cards.clone(),
        rel: e.rel.clone(),
        kind: e.kind.clone(),
        endk: e.endk.clone(),
        vark: e.vark.clone(),
        srcs: e.srcs.clone(),
        ebor: e.rel.iter().map(|&r| !is_owned(e.cards[r].id)).collect(),
        cvm: e.srcs.iter().map(cv_mask).collect(),
        plain: e.plain_ids.iter().zip(&e.plain_names).map(|(&id, n)| (id, n.clone(), card_of[&id])).collect(),
        locked: e.locked.clone(),
        allowed_borrow: usize::from(dj.borrow),
        kinds_all,
        keys: std::cell::RefCell::default(),
        red: std::cell::RefCell::default(),
        gmc: std::cell::RefCell::default(),
        nodes: std::cell::Cell::default(),
        legt: std::cell::RefCell::default(),
        counts: std::cell::OnceCell::default(),
        lockbits: std::cell::OnceCell::default(),
        sbcfg: std::cell::RefCell::default(),
        sbrows: std::cell::RefCell::default(),
        domc: std::cell::RefCell::default(),
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
/// each own a block of `block_len` f64s (built from 1.0), in group order; a group outside it has none (reading it panics, as
/// an empty table's read did)
pub(super) struct GTab {
    /// Packed block index per group; u32::MAX marks a group omitted by `need`.
    slot: Vec<u32>,
    block_len: usize,
    data: Vec<f64>,
}
impl GTab {
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
pub(super) fn gm_w(b: &[f64], nu: usize, r: usize) -> &[f64] {
    &b[r * 2 * nu..(r * 2 + 1) * nu]
}
#[inline(always)]
pub(super) fn gm_g(b: &[f64], nu: usize, r: usize) -> &[f64] {
    &b[(r * 2 + 1) * nu..(r + 1) * 2 * nu]
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
