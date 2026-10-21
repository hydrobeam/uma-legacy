//! The space of legal decks (`DeckSpace`): entries by chain-kind key, legality tables and caps.
use super::*;

thread_local! { pub(super) static UCHK: std::cell::Cell<(usize, f64, usize)> = std::cell::Cell::new((0, 0.0, 0)); pub(super) static SPT: std::cell::Cell<(f64, f64, usize, usize)> = std::cell::Cell::new((0.0, 0.0, 0, 0)); }
/// Every legal deck of one trainee, without the skyline's pruning (which compares product odds only). Used to make
/// the per-uma search exact: decks are enumerated by chain-kind key, like the skyline search, with an upper bound on
/// a partial deck's product value (each remaining slot of a kind group at that group's best miss chances per target,
/// ignoring the one-per-character rule), so only the decks that can reach a threshold are built.
pub(super) struct DeckSpace<'a> {
    pub(super) dj: &'a DeckJob, pub(super) unc: Vec<u32>, pub(super) sk: &'a SkillsRaw,
    pub(super) cards: Vec<&'a Card>,
    // entries (as in run_deck_relaxed, before card dominance drops any): card index, chain kind, early end, variant
    pub(super) rel: Vec<usize>, pub(super) kind: Vec<Kind>, pub(super) endk: Vec<u8>, pub(super) vark: Vec<u32>, pub(super) srcs: Vec<Src>, pub(super) ebor: Vec<bool>,
    // per entry, `cv_mask` of its source (the class caps)
    pub(super) cvm: Vec<u64>,
    // pal characters with no relevant card: (id, name, card)
    pub(super) plain: Vec<(u32, String, &'a Card)>,
    pub(super) locked: Vec<usize>, pub(super) allowed_borrow: usize,
    // every chain kind of any entry plus fillers (2, 2): key codes are counts in base 7 over this list
    pub(super) kinds_all: Vec<Kind>,
    pub(super) keys: std::cell::RefCell<crate::FxMap<u64, std::rc::Rc<(Vec<f64>, Vec<f64>, Vec<u32>)>>>,
    pub(super) red: std::cell::RefCell<crate::FxMap<Vec<usize>, (std::rc::Rc<Vec<usize>>, usize)>>,
    pub(super) gmc: std::cell::RefCell<crate::FxMap<(usize, u64), std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>>>>,
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
pub(super) struct LegalTabs { pub(super) over: bool, pub(super) ebit: Vec<u128>, pub(super) by_kind: Vec<Vec<usize>>, pub(super) pal_ents: Vec<usize>, pub(super) glk: Vec<u8>, pub(super) nplain: usize }
#[derive(Clone)]
pub(super) struct Sel { pub(super) ents: Vec<usize>, pub(super) plain: usize, pub(super) filler: usize, pub(super) dw: Vec<f64>, pub(super) dg: Vec<f64> }
impl<'a> DeckSpace<'a> {
    /// per-entry miss chances (white given no gold, gold), entry-major, under the chain odds of key `code`
    /// per entry and target the miss chances (white, gold) under the key, and per entry the targets it touches (bit t:
    /// a miss chance other than exactly 1.0)
    pub(super) fn key_miss(&self, code: u64) -> std::rc::Rc<(Vec<f64>, Vec<f64>, Vec<u32>)> {
        if let Some(r) = self.keys.borrow().get(&code) { return r.clone(); }
        let nu = self.unc.len(); let n = self.rel.len();
        let mut ks: Vec<Kind> = vec![]; let mut c = code;
        for &kd in &self.kinds_all { for _ in 0..(c % 7) { ks.push(kd); } c /= 7; }
        let marg = chain_marginals(&ks);
        let empty: Vec<f64> = vec![];
        let mut mw = vec![1.0f64; n * nu]; let mut mg = vec![1.0f64; n * nu];
        for j in 0..n { src_miss(&self.srcs[j], marg.get(&self.kind[j]).unwrap_or(&empty), &mut mw[j * nu..(j + 1) * nu], &mut mg[j * nu..(j + 1) * nu]); }
        assert!(nu <= 32);
        let tm: Vec<u32> = (0..n).map(|j| (0..nu).filter(|&t| mw[j * nu + t] != 1.0 || mg[j * nu + t] != 1.0).fold(0u32, |m, t| m | 1 << t)).collect();
        let r = std::rc::Rc::new((mw, mg, tm));
        self.keys.borrow_mut().insert(code, r.clone());
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
        let relt: Vec<bool> = (0..nu).map(|k| us.iter().any(|&u| model.opts[u].iter().any(|(st, _)| st[k] < 2))).collect();
        let pg = |g: &Group| -> Group { g.iter().map(|(p, o)| (*p, o.iter().copied().filter(|x| relt[x.0]).collect::<Vec<_>>())).collect() };
        let proj = |x: &Src| -> String {
            let tr: Vec<(usize, u64)> = x.train.iter().filter(|t| relt[t.0]).map(|t| (t.0, t.1.to_bits())).collect();
            let fx: Vec<Group> = x.fixed.iter().map(|g| pg(g)).filter(|g| g.iter().any(|o| !o.1.is_empty())).collect();
            let ch: Vec<(u8, Group)> = x.chains.iter().map(|(l, g)| (*l, pg(g))).filter(|(_, g)| g.iter().any(|o| !o.1.is_empty())).collect();
            let bits = |g: &Group| -> Vec<(u64, Vec<(usize, u8)>)> { g.iter().map(|(p, o)| (p.to_bits(), o.clone())).collect() };
            format!("{:?}|{:?}|{:?}", tr, fx.iter().map(bits).collect::<Vec<_>>(), ch.iter().map(|(l, g)| (*l, bits(g))).collect::<Vec<_>>())
        };
        let legal: Vec<usize> = (0..self.rel.len()).filter(|&j| ids.iter().all(|&id| !self.cards[self.rel[j]].blocks(id))).collect();
        let keys: Vec<String> = legal.iter().map(|&j| proj(&self.srcs[j])).collect();
        let empty = proj(&Src::default());
        let is_locked = |j: usize| self.locked.contains(&self.rel[j]);
        let mut keep: Vec<usize> = vec![];
        let mut seen: Vec<(u32, Kind, &String)> = vec![];
        let mut used_chara: Vec<u32> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] == empty && !is_locked(j) { continue; }
            let ch = self.cards[self.rel[j]].chara;
            if !is_locked(j) && seen.iter().any(|x| x.0 == ch && x.1 == self.kind[j] && x.2 == &keys[i]) { continue; }
            seen.push((ch, self.kind[j], &keys[i])); keep.push(j); used_chara.push(ch);
        }
        let mut cls: Vec<((Kind, bool, bool), Vec<usize>)> = vec![];
        for (i, &j) in legal.iter().enumerate() {
            if keys[i] != empty || is_locked(j) { continue; }
            let c = (self.kind[j], self.cards[self.rel[j]].pal, self.ebor[j]);
            match cls.iter_mut().find(|x| x.0 == c) { Some(x) => x.1.push(j), None => cls.push((c, vec![j])) }
        }
        let size = self.dj.deck_size;
        for (_, mem) in &cls {
            let mut pick: Vec<usize> = vec![]; let mut chs: Vec<u32> = vec![];
            for pass in 0..2 {
                for &j in mem {
                    if pick.len() >= size { break; }
                    let ch = self.cards[self.rel[j]].chara;
                    if chs.contains(&ch) || pick.contains(&j) { continue; }
                    if pass == 0 && used_chara.contains(&ch) { continue; }
                    pick.push(j); chs.push(ch);
                }
            }
            keep.extend(pick);
        }
        keep.sort();
        keep
    }
    /// (reduced entries, an id per distinct list for other caches): trainees whose lists are equal (a deck class) share
    /// the id, so what is cached by it (`gm_cached`, `legal_tabs`) is built once for them all
    pub(super) fn reduced_cached(&self, model: &Model, us: &[usize], ids: &[u32]) -> (std::rc::Rc<Vec<usize>>, usize) {
        if let Some(r) = self.red.borrow().get(us) { return r.clone(); }
        let list = self.reduced(model, us, ids);
        let same = self.red.borrow().values().find(|v| *v.0 == list).cloned();
        let r = same.unwrap_or_else(|| { let id = self.red.borrow().values().map(|v| v.1 + 1).max().unwrap_or(0); (std::rc::Rc::new(list), id) });
        self.red.borrow_mut().insert(us.to_vec(), r.clone());
        r
    }
    /// (`need`: bit i set for the groups the key's configurations use; the others are left empty)
    pub(super) fn gm_cached(&self, u: usize, code: u64, groups: &[&Vec<usize>], need: u64, mw: &[f64], mg: &[f64], nu: usize, size: usize) -> std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>> {
        if let Some(r) = self.gmc.borrow().get(&(u, code)) { return r.clone(); }
        let r = std::rc::Rc::new(groups.iter().enumerate().map(|(gi, l)| {
            if need >> gi & 1 == 0 { return vec![]; }
            let mut pre = vec![(vec![1.0f64; nu], vec![1.0f64; nu]); size + 1];
            for t in 0..nu {
                let mut a: Vec<f64> = l.iter().map(|&j| mw[j * nu + t]).collect(); a.sort_by(|x, y| x.partial_cmp(y).unwrap());
                let mut b: Vec<f64> = l.iter().map(|&j| mg[j * nu + t]).collect(); b.sort_by(|x, y| x.partial_cmp(y).unwrap());
                for r in 1..=size { pre[r].0[t] = pre[r - 1].0[t] * a.get(r - 1).copied().unwrap_or(1.0); pre[r].1[t] = pre[r - 1].1[t] * b.get(r - 1).copied().unwrap_or(1.0); }
            }
            pre }).collect::<Vec<_>>());
        self.gmc.borrow_mut().insert((u, code), r.clone());
        r
    }
    /// `search`'s tables for one legal entry set (reduced list `legal`, its id, the trainees' ids), built on first use
    pub(super) fn legal_tabs(&self, legal: &[usize], set_id: usize, ids: &[u32]) -> std::rc::Rc<LegalTabs> {
        let nplain = self.legal_plain(ids).len();
        let key = (set_id, nplain);
        if let Some(r) = self.legt.borrow().get(&key) { return r.clone(); }
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &j in legal { let n = chara_ix.len(); chara_ix.entry(self.cards[self.rel[j]].chara).or_insert(n); }
        let over = chara_ix.len() > 128;
        let ebit: Vec<u128> = if over { vec![] } else { (0..self.rel.len()).map(|j| chara_ix.get(&self.cards[self.rel[j]].chara).map_or(0, |&c| 1u128 << c)).collect() };
        let nk = self.kinds_all.len();
        let by_kind: Vec<Vec<usize>> = (0..nk).map(|i| legal.iter().copied().filter(|&j| self.kind[j] != (0, 0, 0) && self.kind[j] == self.kinds_all[i]).collect()).collect();
        let pal_ents: Vec<usize> = legal.iter().copied().filter(|&j| self.kind[j] == (0, 0, 0)).collect();
        // locked cards each group (by_kind, then pal_ents) can hold
        let lb = |j: usize| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0u8, |b| 1u8 << b);
        let glk: Vec<u8> = by_kind.iter().chain(std::iter::once(&pal_ents)).map(|l| l.iter().fold(0u8, |m, &j| m | lb(j))).collect();
        let r = std::rc::Rc::new(LegalTabs { over, ebit, by_kind, pal_ents, glk, nplain });
        self.legt.borrow_mut().insert(key, r.clone());
        r
    }
    /// every count vector over `kinds_all` (each entry the number of cards of that kind, summing to at most deck_size)
    pub(super) fn counts_all(&self) -> std::rc::Rc<Vec<Vec<usize>>> {
        self.counts.get_or_init(|| {
            let (nk, size) = (self.kinds_all.len(), self.dj.deck_size);
            let mut counts: Vec<Vec<usize>> = vec![vec![]];
            for _ in 0..nk { counts = counts.into_iter().flat_map(|c| { let used: usize = c.iter().sum(); (0..=size - used).map(move |x| { let mut d = c.clone(); d.push(x); d }) }).collect(); }
            std::rc::Rc::new(counts)
        }).clone()
    }
    /// per entry, its locked card's bit (0 if none); and every locked card's bit
    pub(super) fn lock_bits(&self) -> &(Vec<u8>, u8) {
        self.lockbits.get_or_init(|| {
            let lbit: Vec<u8> = (0..self.rel.len()).map(|j| self.locked.iter().position(|&l| l == self.rel[j]).map_or(0, |b| 1u8 << b)).collect();
            let all_locked: u8 = if self.locked.is_empty() { 0 } else { ((1u16 << self.locked.len()) - 1) as u8 };
            (lbit, all_locked)
        })
    }
    /// the class caps (`EXACT_CAPS`, or all at AUDIT_CAP under `cap_audit`)
    pub(crate) fn caps(&self) -> [f64; 4] { if self.dj.cap_audit { [AUDIT_CAP; 4] } else { test_caps().unwrap_or(EXACT_CAPS) } }
    /// most decks one band search may return: EXACT_CAP, ten times that under `cap_audit` (its bands are far larger)
    pub(crate) fn band_cap(&self) -> usize { if self.dj.cap_audit { 10 * EXACT_CAP } else { EXACT_CAP } }
    /// the highest class cap (EXACT_RHO unless `cap_audit`)
    pub(crate) fn rho_top(&self) -> f64 { self.caps()[3] }
    /// a deck's structure class (`cap_class`)
    pub(crate) fn sel_class(&self, x: &Sel) -> usize { cap_class(x.ents.iter().fold(0u64, |m, &j| m | self.cvm[j])) }
    /// a deck's cap under the ceiling rho: its class's cap, at most rho
    pub(crate) fn sel_cap(&self, x: &Sel, rho: f64) -> f64 { self.caps()[self.sel_class(x)].min(rho) }
    pub(super) fn legal_plain(&self, ids: &[u32]) -> Vec<&(u32, String, &'a Card)> { self.plain.iter().filter(|x| ids.iter().all(|&id| !x.2.blocks(id))).collect() }
    pub(super) fn pdeck(&self, x: &Sel, ids: &[u32]) -> PDeck {
        let srcb = Sources { dj: self.dj, unc: &self.unc };
        let lp = self.legal_plain(ids);
        let ci: Vec<usize> = x.ents.iter().map(|&j| self.rel[j]).collect();
        let borrowed = x.ents.iter().copied().find(|&j| self.ebor[j]).map(|j| self.rel[j]);
        let mut names: Vec<String> = x.ents.iter().map(|&j| {
            let c = self.cards[self.rel[j]]; let e = self.endk[j];
            let mut n = c.name.clone();
            let lb = srcb.label(c, (e > 0).then_some(e), self.vark[j], self.sk);
            if !lb.is_empty() { n = format!("{} ({})", n, lb); }
            if e > 0 { n = format!("{} (end chain at link {})", n, e); }
            if Some(self.rel[j]) == borrowed { n = format!("{} (borrowed)", n); }
            n
        }).collect();
        names.extend(lp.iter().take(x.plain).map(|p| p.1.clone()));
        for _ in 0..x.filler { names.push("(any SR without target hints)".into()); }
        PDeck { names, ids: ci.iter().map(|&i| self.cards[i].id).collect(), ends: x.ents.iter().map(|&j| self.endk[j]).collect(),
                vars: x.ents.iter().map(|&j| self.vark[j]).collect(),
                choices: x.ents.iter().map(|&j| srcb.label(self.cards[self.rel[j]], (self.endk[j] > 0).then_some(self.endk[j]), self.vark[j], self.sk)).collect(),
                plain_ids: lp.iter().take(x.plain).map(|p| p.0).collect(),
                pal_names: ci.iter().filter(|&&i| self.cards[i].pal).map(|&i| self.cards[i].name.clone()).chain(lp.iter().take(x.plain).map(|p| p.1.clone())).collect(),
                pals: x.ents.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + x.plain,
                borrowed: borrowed.map(|i| self.cards[i].name.clone()), dw: x.dw.clone(), dg: x.dg.clone() }
    }
}

pub(super) fn deck_space<'a>(cr: &'a CardsRaw, dj: &'a DeckJob, sk: &'a SkillsRaw, unc: &[u32], e: &Entries<'a>) -> DeckSpace<'a> {
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let mut kinds_all: Vec<Kind> = e.kind.iter().copied().filter(|&k| k != (0, 0, 0)).collect();
    kinds_all.push((2, 2, 0)); kinds_all.sort(); kinds_all.dedup();
    DeckSpace { dj, unc: unc.to_vec(), sk, cards: e.cards.clone(), rel: e.rel.clone(), kind: e.kind.clone(), endk: e.endk.clone(), vark: e.vark.clone(),
        srcs: e.srcs.clone(), ebor: e.rel.iter().map(|&r| !is_owned(e.cards[r].id)).collect(), cvm: e.srcs.iter().map(cv_mask).collect(),
        plain: e.plain_ids.iter().zip(&e.plain_names).map(|(&id, n)| (id, n.clone(), card_of[&id])).collect(),
        locked: e.locked.clone(), allowed_borrow: if dj.borrow { 1 } else { 0 }, kinds_all, keys: Default::default(), red: Default::default(), gmc: Default::default(), nodes: Default::default(),
        legt: Default::default(), counts: Default::default(), lockbits: Default::default(), sbcfg: Default::default(), sbrows: Default::default(), domc: Default::default() }
}
