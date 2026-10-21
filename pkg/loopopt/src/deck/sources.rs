//! Target sources of a deck entry: outcome groups, training hints and chain links per card, the event data merged into
//! a job, and the deck entries of a job.
use super::*;

/// One random event outcome per firing: (chance, target rewards (target, 1 white | 2 gold)) per outcome. The chances
/// sum to at most 1 (the rest gives nothing); a reward with chance p on its own is [(p, [(k, lvl)])].
pub(super) type Group = Vec<(f64, Vec<(usize, u8)>)>;
/// Per target a group touches: (k, P(gold for k), P(any hint for k)).
pub(super) fn group_gh(g: &Group) -> Vec<(usize, f64, f64)> {
    let mut v: Vec<(usize, f64, f64)> = vec![];
    for (w, out) in g {
        let mut seen: Vec<(usize, u8)> = vec![];
        for &(k, lvl) in out { match seen.iter_mut().find(|x| x.0 == k) { Some(x) => x.1 = x.1.max(lvl), None => seen.push((k, lvl)) } }
        for (k, lvl) in seen {
            let e = match v.iter().position(|x| x.0 == k) { Some(i) => &mut v[i], None => { v.push((k, 0.0, 0.0)); v.last_mut().unwrap() } };
            if lvl >= 2 { e.1 += w; }
            e.2 += w;
        }
    }
    v.sort_by_key(|x| x.0);
    v
}

/// Proven upper bound on the exact P(all) of trainee `u` with one deck, per inheritance chance in `qs` (fixed factor
/// included), for decks the bound covers; None otherwise. Every card is grouped with the targets its chain feeds;
/// each group's factor is a rising function of that card's progress alone. Given N = n, when the progress vector is
/// negatively upper-orthant dependent (`key_level`, checked exactly per key), the expected product of such
/// functions of distinct coordinates is at most the product of their expectations (write each as a positive
/// combination of indicators {X_i >= t}). Event outcomes and training hints are independent of the allocation, and
/// exclusive outcomes only lower the product (outcome indicators are negatively associated), so each target's
/// outcomes are taken at their marginal chances.
///   U = sum_n P(N = n) prod_cards E[prod_{k fed by its chain} phi_k(X_card) | n] prod_{other k} phi_k
/// Not covered (None): a target fed by two cards' chains, or a key that fails the orthant check.
/// A group's target indicators (which targets one firing hints) are negatively upper-orthant dependent: for every set
/// S of its targets, P(all of S hinted) <= prod over S of P(k hinted). True for outcomes that each give one target
/// (they exclude each other); not for an outcome giving two targets together. Training draws over several targets
/// are checked the same way. Cached per group (by its bits).
/// Whether a group's per-target hint levels are negatively associated, as the state-conditioned bound needs: every
/// outcome hints at most one target (one draw: a one-trial multinomial is negatively associated, and so are increasing
/// functions of disjoint sets of its outcomes), or it touches two targets and their levels are negatively quadrant
/// dependent, P(A >= a, B >= b) <= P(A >= a) P(B >= b) for a, b in {white, gold} (for two variables negative quadrant
/// dependence is negative association). Three or more targets with an outcome hinting several: not covered.
pub(super) fn group_assoc(g: &Group) -> bool {
    if g.iter().all(|(_, o)| o.iter().all(|x| x.0 == o[0].0)) { return true; }
    let mut ts: Vec<usize> = g.iter().flat_map(|(_, o)| o.iter().map(|x| x.0)).collect(); ts.sort(); ts.dedup();
    if ts.len() != 2 { return false; }
    let lv = |o: &[(usize, u8)], t: usize| o.iter().filter(|x| x.0 == t).map(|x| if x.1 >= 2 { 2u8 } else { 1u8 }).max().unwrap_or(0);
    for a in 1..=2u8 { for b in 1..=2u8 {
        let pa: f64 = g.iter().filter(|(_, o)| lv(o, ts[0]) >= a).map(|x| x.0).sum();
        let pb: f64 = g.iter().filter(|(_, o)| lv(o, ts[1]) >= b).map(|x| x.0).sum();
        let pj: f64 = g.iter().filter(|(_, o)| lv(o, ts[0]) >= a && lv(o, ts[1]) >= b).map(|x| x.0).sum();
        if pj > pa * pb * (1.0 + 1e-12) + 1e-15 { return false; }
    } }
    true
}
pub(super) fn group_nuod(g: &Group) -> bool {
    thread_local! { static GN: std::cell::RefCell<crate::FxMap<Vec<(u64, Vec<(usize, u8)>)>, bool>> = std::cell::RefCell::new(Default::default()); }
    let key: Vec<(u64, Vec<(usize, u8)>)> = g.iter().map(|(p, o)| (p.to_bits(), o.clone())).collect();
    if let Some(r) = GN.with(|c| c.borrow().get(&key).copied()) { return r; }
    let mut ts: Vec<usize> = g.iter().flat_map(|(_, o)| o.iter().map(|x| x.0)).collect(); ts.sort(); ts.dedup();
    // per outcome, the targets it hints as a bit set
    let outs: Vec<(f64, u32)> = g.iter().map(|(p, o)| (*p, o.iter().fold(0u32, |m, x| m | 1 << ts.iter().position(|&t| t == x.0).unwrap()))).collect();
    let marg: Vec<f64> = (0..ts.len()).map(|i| outs.iter().filter(|o| o.1 >> i & 1 == 1).map(|o| o.0).sum()).collect();
    let ok = ts.len() > 20 || (1u32..1 << ts.len()).filter(|s| s.count_ones() >= 2).all(|s| {
        let joint: f64 = outs.iter().filter(|o| o.1 & s == s).map(|o| o.0).sum();
        joint <= (0..ts.len()).filter(|i| s >> i & 1 == 1).map(|i| marg[i]).product::<f64>() * (1.0 + 1e-12) + 1e-15
    });
    GN.with(|c| c.borrow_mut().insert(key, ok));
    ok
}
/// Whether a group's per-target hint levels (Y_k: 0 none, 1 white, 2 gold, the best level its outcome gives k) are
/// negatively upper-orthant dependent at every level: for every set S of two or more of its targets and every s in
/// {1, 2}^S, P(Y_k >= s_k for all k in S) <= prod_S P(Y_k >= s_k). What the cap-free bound (`cf_ok`) needs: a nonnegative
/// rising function of one Y_k is a nonnegative combination of 1, 1{Y_k >= 1} and 1{Y_k >= 2}, so the expected product
/// of such functions of distinct targets is at most the product of their expectations. Implied by `group_assoc` (two
/// targets: the same test; single-target outcomes: every joint term is 0). Over 6 targets: not checked (false).
pub(super) fn group_nuod_lvl(g: &Group) -> bool {
    let mut ts: Vec<usize> = g.iter().flat_map(|(_, o)| o.iter().map(|x| x.0)).collect(); ts.sort(); ts.dedup();
    if ts.len() < 2 { return true; }
    if ts.len() > 6 { return false; }
    let m = ts.len();
    // per outcome: its chance and each target's level
    let outs: Vec<(f64, Vec<u8>)> = g.iter().map(|(p, o)| (*p, ts.iter().map(|&t| o.iter().filter(|x| x.0 == t).map(|x| if x.1 >= 2 { 2u8 } else { 1u8 }).max().unwrap_or(0)).collect())).collect();
    let marg = |i: usize, s: u8| -> f64 { outs.iter().filter(|o| o.1[i] >= s).map(|o| o.0).sum() };
    // every assignment of a level 0 (not in S), 1 or 2 to each target, base 3
    let mut a = vec![0u8; m];
    loop {
        let mut i = 0; while i < m { a[i] += 1; if a[i] < 3 { break; } a[i] = 0; i += 1; }
        if i == m { return true; }
        if a.iter().filter(|&&x| x > 0).count() < 2 { continue; }
        let joint: f64 = outs.iter().filter(|o| (0..m).all(|i| o.1[i] >= a[i])).map(|o| o.0).sum();
        let prod: f64 = (0..m).filter(|&i| a[i] > 0).map(|i| marg(i, a[i])).product();
        if joint > prod * (1.0 + 1e-12) + 1e-15 { return false; }
    }
}

/// Target sources of one deck entry (a card, a card whose chain the player ends early, or a card taking a given
/// choice where an event offers targets on different choices): training hints (target, chance), outcome groups that
/// do not depend on the chain, and chain outcome groups with their link, sorted by link. `fm` / `cm` are the same
/// per target as miss factors (1 - P(gold), 1 - P(any hint)), the chain ones per link in link order.
#[derive(Clone, Default)]
pub(super) struct Src { pub(super) train: Vec<(usize, f64)>, pub(super) fixed: Vec<Group>, pub(super) chains: Vec<(u8, Group)>,
             pub(super) fm: Vec<(usize, f64, f64)>, pub(super) cm: Vec<(usize, Vec<(u8, f64, f64)>)>,
             /// a group of it touching two or more targets is not negatively upper-orthant dependent (`group_nuod`):
             /// `upper_na` does not cover a deck with it
             pub(super) not_nuod: bool,
             /// an outcome of one of its groups hints two or more targets at once (the state-conditioned bound, which
             /// takes each target's outcomes at their marginal chances, does not cover a deck with it)
             pub(super) multi: bool,
             /// every group of it passes `group_nuod_lvl`: the state-conditioned bound holds for it without any cap (the
             /// cap-free deck search, `Env::best_deck_cf`, covers only decks of such entries)
             pub(super) cf_ok: bool }
impl Src {
    pub(super) fn finish(mut self) -> Self {
        self.chains.sort_by_key(|x| x.0);
        self.fm = self.fixed.iter().flat_map(|g| group_gh(g)).map(|(k, g, h)| (k, 1.0 - g, 1.0 - h)).collect();
        let mut cm: Vec<(usize, Vec<(u8, f64, f64)>)> = vec![];
        for (l, g) in &self.chains {
            for (k, pg, ph) in group_gh(g) {
                let e = match cm.iter().position(|x| x.0 == k) { Some(i) => &mut cm[i].1, None => { cm.push((k, vec![])); &mut cm.last_mut().unwrap().1 } };
                match e.last_mut() { Some(x) if x.0 == *l => { x.1 *= 1.0 - pg; x.2 *= 1.0 - ph; } _ => e.push((*l, 1.0 - pg, 1.0 - ph)) }
            }
        }
        cm.sort_by_key(|x| x.0);
        self.cm = cm;
        self.not_nuod = self.fixed.iter().chain(self.chains.iter().map(|x| &x.1)).any(|g| group_gh(g).len() >= 2 && !group_nuod(g));
        self.multi = self.fixed.iter().chain(self.chains.iter().map(|x| &x.1)).any(|g| !group_assoc(g));
        self.cf_ok = self.fixed.iter().chain(self.chains.iter().map(|x| &x.1)).all(|g| group_nuod_lvl(g));
        self
    }
    pub(super) fn is_empty(&self) -> bool { self.train.is_empty() && self.fixed.is_empty() && self.chains.is_empty() }
}

/// Multiply an entry's miss chances into mw / mg; m[l - 1] = P(chain link l fires). mg gets P(no gold hint) and mw
/// P(no white hint | no gold hint) = P(no hint) / P(no gold hint), both exact for the entry: its sources are
/// independent of each other, while a skill's gold and white from one event (e.g. on different outcomes) are not.
/// Links fire in order, so a skill offered at several links comes from the earliest one that fires:
/// P(miss) = 1 - sum_l P(link l fires) * (P(not from links < l) - P(not from links <= l)).
/// Both misses fall with every m[l], so the lowest possible link odds bound them from above.
pub(super) fn src_miss(s: &Src, m: &[f64], mw: &mut [f64], mg: &mut [f64]) {
    let mut eg = [1.0f64; 32]; let mut eh = [1.0f64; 32];
    for &(k, p) in &s.train { eh[k] *= 1.0 - p; }
    for &(k, ng, nh) in &s.fm { eg[k] *= ng; eh[k] *= nh; }
    for (k, links) in &s.cm {
        let (mut cg, mut ch, mut xg, mut xh) = (1.0f64, 1.0f64, 1.0f64, 1.0f64);
        for &(l, ng, nh) in links {
            let ml = m.get(l as usize - 1).copied().unwrap_or(0.0);
            let (ncg, nch) = (cg * ng, ch * nh);
            xg -= ml * (cg - ncg); xh -= ml * (ch - nch); cg = ncg; ch = nch;
        }
        eg[*k] *= xg; eh[*k] *= xh;
    }
    for k in 0..mw.len() { mg[k] *= eg[k]; mw[k] *= if eg[k] > 0.0 { eh[k] / eg[k] } else { 1.0 }; }
}

/// Merge a card-event cache (card_events.json) into a deck job: skill names -> (group, level) via the skills table.
pub(crate) fn apply_events_db(v: &mut Value, sk: &SkillsRaw, db: &Value) { apply_event_lists(v, &event_lists(sk, db)); }

/// The job lists a card-event cache adds, in `EVENT_KEYS` order: the same for every job (the page's worker builds
/// them once, at init).
pub(crate) struct EventLists([Vec<Value>; 8]);
const EVENT_KEYS: [&str; 8] = ["events", "events_p", "odds_events", "end_links", "ends", "stop_risk", "alts", "alt_w"];
/// Each list after the job's own entries of it, in their order.
pub(crate) fn apply_event_lists(v: &mut Value, l: &EventLists) {
    for (k, add) in EVENT_KEYS.iter().zip(&l.0) {
        let mut a: Vec<Value> = match v[*k].take() { Value::Array(a) => a, _ => vec![] };
        a.extend(add.iter().cloned());
        v[*k] = Value::Array(a);
    }
}
pub(crate) fn event_lists(sk: &SkillsRaw, db: &Value) -> EventLists {
    let look = |name: &str| sk.skills.iter().find_map(|(g, w, gd)| if w == name { Some((*g, 1u8)) } else if gd.as_deref() == Some(name) { Some((*g, 2u8)) } else { None });
    let [mut ev, mut evp, mut od, mut el, mut en, mut sr, mut al, mut aw]: [Vec<Value>; 8] = Default::default();
    if let Some(cards) = db["cards"].as_object() {
        for (id, c) in cards {
            let Ok(id) = id.parse::<u32>() else { continue };
            for link in c["chain"].as_array().into_iter().flatten() {
                let l = link[0].as_u64().unwrap_or(0);
                for n in link[1].as_array().into_iter().flatten() {
                    let (name, p) = if let Some(a) = n.as_array() { (a[0].as_str().unwrap_or(""), a[1].as_f64().unwrap_or(0.0)) } else { (n.as_str().unwrap_or(""), 1.0) };
                    if let Some((g, lv)) = look(name) { if p >= 1.0 { ev.push(json!([id, g, lv, l])); } else { evp.push(json!([id, g, lv, l, p])); } }
                }
            }
            for link in c["end"].as_array().into_iter().flatten() {
                let l = link[0].as_u64().unwrap_or(0);
                el.push(json!([id, l]));
                for n in link[1].as_array().into_iter().flatten() {
                    let (name, p) = if let Some(a) = n.as_array() { (a[0].as_str().unwrap_or(""), a[1].as_f64().unwrap_or(0.0)) } else { (n.as_str().unwrap_or(""), 1.0) };
                    if let Some((g, lv)) = look(name) { en.push(json!([id, l, g, lv, p])); }
                }
            }
            for r in c["stop_risk"].as_array().into_iter().flatten() { sr.push(json!([id, r[0], r[1], r[2]])); }
            for a in c["alts"].as_array().into_iter().flatten() {
                let chs: Vec<Vec<Vec<(u32, u8)>>> = a[1].as_array().into_iter().flatten().map(|ch| ch.as_array().into_iter().flatten()
                    .map(|out| out.as_array().into_iter().flatten().filter_map(|n| look(n.as_str().unwrap_or(""))).collect()).collect()).collect();
                al.push(json!([id, a[0], chs]));
            }
            for a in c["alt_w"].as_array().into_iter().flatten() { aw.push(json!([id, a[0], a[1]])); }
            for n in c["always"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(n.as_str().unwrap_or("")) { ev.push(json!([id, g, lv, 0])); } }
            for e in c["chance"].as_array().into_iter().flatten() { if let Some((g, lv)) = look(e[0].as_str().unwrap_or("")) { od.push(json!([id, g, lv, e[1].as_f64().unwrap_or(0.0)])); } }
        }
    }
    EventLists([ev, evp, od, el, en, sr, al, aw])
}

/// Builds a deck entry's target sources and chain kind from the deck job; shared by the deck search and the exact
/// evaluation of the decks it reports.
pub(super) struct Sources<'a> { pub(super) dj: &'a DeckJob, pub(super) unc: &'a [u32] }
impl<'a> Sources<'a> {
    pub(super) fn ev_for(&self, id: u32) -> Vec<(usize, u8, u8, f64)> {
        let unc = self.unc;
        let mut v: Vec<(usize, u8, u8, f64)> = self.dj.events.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, 1.0))).collect();
        v.extend(self.dj.events_p.iter().filter(|e| e.0 == id).filter_map(|e| unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3, e.4))));
        v
    }
    pub(super) fn odds_for(&self, id: u32) -> Vec<(usize, u8, f64)> { self.dj.odds_events.iter().filter(|e| e.0 == id).filter_map(|e| self.unc.iter().position(|&g| g == e.1).map(|k| (k, e.2, e.3))).collect() }
    /// Training-hint draws per run: base(scenario) x (1 + hint frequency / 100) x (1 + d(scenario) x specialty / 100),
    /// each uniform over the card's skill options still below Lv 5. Per target in the pool: P(hinted) = 1 - hint_miss(1).
    pub(super) fn draws(&self, c: &Card) -> f64 {
        let s = &self.dj.scenario;
        hint_base(s) * (1.0 + c.hf / 100.0) * (1.0 + hint_spec(s) * c.spec / 100.0)
    }
    pub(super) fn train(&self, c: &Card) -> Vec<(usize, f64)> {
        if c.nskill == 0 { return vec![]; }
        let p = 1.0 - hint_miss(1, c.nskill as usize, self.draws(c), c.hinc);
        self.unc.iter().enumerate().filter(|(_, g)| c.whites.contains(g)).map(|(k, _)| (k, p)).collect()
    }
    /// A card's training hints over two or more of its targets as one outcome group: one outcome per non-empty set S
    /// of targets hinted, P(exactly S) = sum over U in S of (-1)^(|S|-|U|) hint_miss(m - |U|) (m targets in the
    /// pool), so the draws that hint one target cannot also hint another. Each target's own chance is unchanged.
    pub(super) fn train_group(&self, c: &Card, tr: &[(usize, f64)]) -> Group {
        let (m, n, h) = (tr.len(), c.nskill as usize, self.draws(c));
        let miss = |unhit: usize| hint_miss(unhit, n, h, c.hinc);   // P(none of `unhit` given targets hinted)
        let mut g: Group = vec![];
        for sm in 1u32..(1 << m) {
            let mut w = 0.0f64; let ssz = sm.count_ones() as i32;
            let mut u = sm;
            loop {                                                    // every subset U of S (including the empty set)
                let usz = u.count_ones() as i32;
                w += if (ssz - usz) % 2 == 0 { 1.0 } else { -1.0 } * miss(m - usz as usize);
                if u == 0 { break; }
                u = (u - 1) & sm;
            }
            g.push((w.max(0.0), (0..m).filter(|i| sm >> i & 1 == 1).map(|i| (tr[i].0, 1u8)).collect()));
        }
        g
    }
    /// early-end options: links where a card has a choice that ends its chain, and the target rewards of that choice
    pub(super) fn end_links(&self, id: u32) -> Vec<u8> { let mut v: Vec<u8> = self.dj.end_links.iter().filter(|e| e.0 == id).map(|e| e.1).collect(); v.sort(); v.dedup(); v }
    pub(super) fn end_rew(&self, id: u32, l: u8) -> Vec<(usize, u8, f64)> { self.dj.ends.iter().filter(|e| e.0 == id && e.1 == l).filter_map(|e| self.unc.iter().position(|&g| g == e.2).map(|k| (k, e.3, e.4))).collect() }
    /// The choices worth taking at an event listed in `alts` (None: not listed), as outcome groups over the targets:
    /// those with a target that no other choice beats on every target (P(gold) and P(any hint)); of two with the same
    /// odds, both are kept unless their outcomes are the same (then the first).
    pub(super) fn choices(&self, id: u32, link: u8) -> Option<Vec<Group>> {
        let a = self.dj.alts.iter().find(|a| a.0 == id && a.1 == link)?;
        let w = self.dj.alt_w.iter().find(|w| w.0 == id && w.1 == link).map(|w| &w.2);   // measured shares, else equal
        let nu = self.unc.len();
        let gs: Vec<Group> = a.2.iter().enumerate().map(|(ci, ch)| {
            let n = ch.len().max(1) as f64;
            ch.iter().enumerate().map(|(oi, out)| {
                let mut r: Vec<(usize, u8)> = out.iter().filter_map(|&(g, lv)| self.unc.iter().position(|&x| x == g).map(|k| (k, lv))).collect();
                r.sort(); r.dedup();
                (w.and_then(|w| w.get(ci)).and_then(|c| c.get(oi)).copied().unwrap_or(1.0 / n), r)
            }).filter(|(_, r)| !r.is_empty()).collect()
        }).collect();
        let prof: Vec<Vec<(f64, f64)>> = gs.iter().map(|g| { let mut p = vec![(0.0, 0.0); nu]; for (k, pg, ph) in group_gh(g) { p[k] = (pg, ph); } p }).collect();
        let beats = |j: usize, i: usize| prof[j].iter().zip(&prof[i]).all(|(a, b)| a.0 >= b.0 && a.1 >= b.1) && (prof[j] != prof[i] || (gs[j] == gs[i] && j < i));
        Some((0..gs.len()).filter(|&i| !gs[i].is_empty() && !(0..gs.len()).any(|j| j != i && !gs[j].is_empty() && beats(j, i))).map(|i| gs[i].clone()).collect())
    }
    pub(super) fn in_range(link: u8, stop: Option<u8>) -> bool { match stop { Some(st) => link < st, None => true } }
    /// links (before the entry's stop) where two or more choices are worth taking, with how many
    pub(super) fn var_links(&self, c: &Card, stop: Option<u8>) -> Vec<(u8, usize)> {
        if c.pal { return vec![]; }
        let mut v: Vec<(u8, usize)> = self.dj.alts.iter().filter(|a| a.0 == c.id && Self::in_range(a.1, stop))
            .filter_map(|a| { let n = self.choices(c.id, a.1).map_or(0, |x| x.len()); (n >= 2).then_some((a.1, n)) }).collect();
        v.sort(); v.dedup(); v
    }
    /// The entry's choice variants: the choice taken at each `var_links` link, 4 bits per link in link order.
    pub(super) fn variants(&self, c: &Card, stop: Option<u8>) -> Vec<u32> {
        let mut out = vec![0u32];
        for (pos, &(_, n)) in self.var_links(c, stop).iter().enumerate() {
            out = out.iter().flat_map(|&v| (0..n.min(16)).map(move |i| v | ((i as u32) << (4 * pos)))).collect();
        }
        out
    }
    pub(super) fn has_alts(&self, id: u32) -> bool { self.dj.alts.iter().any(|a| a.0 == id && self.choices(id, a.1).map_or(false, |x| !x.is_empty())) }
    /// Sources of a card played normally (stop = None) or ending its chain at link `stop` (links before it as usual,
    /// then the ending choice's rewards at that link), taking choice variant `var`.
    pub(super) fn src(&self, c: &Card, stop: Option<u8>, var: u32) -> Src {
        let one = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
        let mut s = Src::default();
        s.fixed = self.odds_for(c.id).into_iter().map(|(k, lvl, p)| one(k, lvl, p)).collect();
        let tr = self.train(c);
        if tr.len() >= 2 { s.fixed.push(self.train_group(c, &tr)); } else { s.train = tr; }
        let alt_links: Vec<u8> = self.dj.alts.iter().filter(|a| a.0 == c.id).map(|a| a.1).collect();
        for (k, lvl, link, f) in self.ev_for(c.id) {
            if link == 0 { s.fixed.push(one(k, lvl, f)); continue; }
            if c.pal || alt_links.contains(&link) || !Self::in_range(link, stop) { continue; }   // pals have no chain slots
            s.chains.push((link, one(k, lvl, f)));
        }
        if !c.pal {
            let vl = self.var_links(c, stop);
            let mut ls = alt_links.clone(); ls.sort(); ls.dedup();
            for l in ls {
                if !Self::in_range(l, stop) { continue; }
                let ch = self.choices(c.id, l).unwrap_or_default();
                let i = vl.iter().position(|x| x.0 == l).map_or(0, |pos| ((var >> (4 * pos)) & 15) as usize);
                if let Some(g) = ch.get(i) { s.chains.push((l, g.clone())); }
            }
        }
        if let Some(st) = stop { for (k, lvl, f) in self.end_rew(c.id, st) { s.chains.push((st, one(k, lvl, f))); } }
        s.finish()
    }
    pub(super) fn kind(&self, c: &Card, stop: Option<u8>) -> Kind {
        if c.pal || c.links == 0 { return (0, 0, 0); }
        let risk: Vec<(u8, u8, u8)> = self.dj.stop_risk.iter().filter(|r| r.0 == c.id).map(|r| (r.1, r.2, r.3)).collect();
        let st = stop.unwrap_or(c.links);
        (c.links, st, stop_risk_code(&risk, st))
    }
    /// "link 1: Uma Stan" for each link where the variant picks between choices ("" when there is none)
    pub(super) fn label(&self, c: &Card, stop: Option<u8>, var: u32, sk: &SkillsRaw) -> String {
        let name = |k: usize, lvl: u8| sk.skills.iter().find(|x| x.0 == self.unc[k]).map(|x| if lvl >= 2 { x.2.clone().unwrap_or(x.1.clone()) } else { x.1.clone() }).unwrap_or_default();
        self.var_links(c, stop).iter().enumerate().map(|(pos, &(l, _))| {
            let i = ((var >> (4 * pos)) & 15) as usize;
            let g = self.choices(c.id, l).unwrap_or_default().get(i).cloned().unwrap_or_default();
            let mut ns: Vec<String> = vec![];
            for (_, out) in &g { for &(k, lvl) in out { let n = name(k, lvl); if !ns.contains(&n) { ns.push(n); } } }
            format!("link {}: {}", l, ns.join(", "))
        }).collect::<Vec<_>>().join("; ")
    }
}

/// Deck entries of a job: per entry the card (`rel`, an index into `cards`, the cards a deck may hold), chain kind,
/// early-end link (0 = played normally), choice variant and sources; the plain pals (owned pal characters with no
/// relevant card, best-rarity card) and the locked cards. `pool_ids`: the umas that may train (plain pals that block
/// none of them come first).
pub(super) struct Entries<'a> { pub(super) cards: Vec<&'a Card>, pub(super) rel: Vec<usize>, pub(super) kind: Vec<Kind>, pub(super) endk: Vec<u8>, pub(super) vark: Vec<u32>, pub(super) srcs: Vec<Src>,
                     pub(super) plain_ids: Vec<u32>, pub(super) plain_names: Vec<String>, pub(super) plain_chars: Vec<u32>, pub(super) locked: Vec<usize> }
pub(super) fn deck_entries<'a>(cr: &'a CardsRaw, dj: &DeckJob, unc: &[u32], pool_ids: &[u32]) -> Result<Entries<'a>, Value> {
    let is_owned = |id: u32| dj.owned_cards.is_empty() || dj.owned_cards.contains(&id) || dj.lock_cards.contains(&id);
    // a card the scenario bans can't be in the deck, owned, borrowed or locked
    let banned: &[u32] = cr.restrict.get(&dj.scenario).map_or(&[], |v| v.as_slice());
    if let Some(c) = dj.lock_cards.iter().find(|id| banned.contains(id)).and_then(|id| cr.cards.iter().find(|c| c.id == *id)) {
        return Err(json!({"error": format!("{} can't be used in {}: the game bans it from that scenario's deck.", c.name, scenario_name(&dj.scenario))}));
    }
    let owned_c = |id: u32| (is_owned(id) || dj.borrow) && !dj.exclude_cards.contains(&id) && !banned.contains(&id);
    // ---- card sources ----
    let srcb = Sources { dj, unc };
    let cards: Vec<&Card> = cr.cards.iter().filter(|c| owned_c(c.id)).collect();
    // deck entries: card index (`rel`), chain kind, early-end link (0 = played normally), choice variant and sources
    let mut rel: Vec<usize> = vec![];
    let mut kind: Vec<Kind> = vec![];
    let mut endk: Vec<u8> = vec![];
    let mut vark: Vec<u32> = vec![];
    let mut srcs: Vec<Src> = vec![];
    let mut plain_names: Vec<String> = vec![];
    let mut plain_ids: Vec<u32> = vec![];
    let mut plain_chars: Vec<u32> = vec![];
    for (i, c) in cards.iter().enumerate() {
        let useful = !srcb.ev_for(c.id).is_empty() || !srcb.odds_for(c.id).is_empty() || !srcb.train(c).is_empty() || srcb.has_alts(c.id);
        let locked = dj.lock_cards.contains(&c.id);
        if c.pal || c.links == 0 {
            // pals, and cards without chain events (R cards): like a pal, an R card takes no chain slot
            if useful || locked { rel.push(i); kind.push((0, 0, 0)); endk.push(0); vark.push(0); srcs.push(srcb.src(c, None, 0)); }
        } else if c.rarity >= dj.min_rarity {
            // one entry per choice variant (an event offering targets on different choices: the player prioritises
            // one); the same card played to end its chain at link l is its own entry too. One character, so a deck
            // holds at most one of a card's entries.
            if useful || locked {
                for v in srcb.variants(c, None) { rel.push(i); kind.push(srcb.kind(c, None)); endk.push(0); vark.push(v); srcs.push(srcb.src(c, None, v)); }
            }
            for l in srcb.end_links(c.id) {
                if l == 0 || l > c.links { continue; }
                for v in srcb.variants(c, Some(l)) {
                    let x = srcb.src(c, Some(l), v);
                    if !x.is_empty() || locked { rel.push(i); kind.push(srcb.kind(c, Some(l))); endk.push(l); vark.push(v); srcs.push(x); }
                }
            }
        }
    }
    // plain pals: owned pal characters with no relevant card in `rel` (best-rarity card name for display).
    // With borrowing, a pal character the player owns no card of can still fill one slot as the borrowed card.
    // A deck with p plain pals uses the first p, so those blocking no uma in the pool come first (a plain pal that
    // blocks a loop member makes the result illegal, and `run_deck` then searches without it).
    let mut plain: Vec<(usize, u32, String)> = vec![];
    for c in cards.iter().filter(|c| c.pal && is_owned(c.id)) {
        if rel.iter().any(|&r| cards[r].chara == c.chara) || plain_chars.contains(&c.chara) { continue; }
        plain_chars.push(c.chara);
        let best = cards.iter().filter(|x| x.pal && x.chara == c.chara && is_owned(x.id)).max_by_key(|x| x.rarity).unwrap();
        plain.push((pool_ids.iter().filter(|&&u| best.blocks(u)).count(), best.id, best.name.clone()));
    }
    plain.sort_by_key(|x| x.0);   // stable: ties keep card order
    for (_, id, name) in plain { plain_ids.push(id); plain_names.push(name); }
    if dj.borrow {
        let mut seen: Vec<u32> = vec![];
        for c in cards.iter() {
            if !c.pal || is_owned(c.id) || plain_chars.contains(&c.chara) || seen.contains(&c.chara) || rel.iter().any(|&r| cards[r].chara == c.chara) { continue; }
            let best = cards.iter().enumerate().filter(|(_, x)| x.pal && x.chara == c.chara).max_by_key(|(_, x)| x.rarity).unwrap().0;
            seen.push(c.chara); rel.push(best); kind.push((0, 0, 0)); endk.push(0); vark.push(0); srcs.push(srcb.src(cards[best], None, 0));
        }
    }
    let mut locked: Vec<usize> = rel.iter().copied().filter(|&r| dj.lock_cards.contains(&cards[r].id)).collect();
    locked.sort(); locked.dedup();
    Ok(Entries { cards, rel, kind, endk, vark, srcs, plain_ids, plain_names, plain_chars, locked })
}
