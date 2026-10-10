//! Chain events: the event count per run, the per-link marginals and the final-state distribution of a deck's chains,
//! and the per-key level tables.
use super::*;

/// Chain events per run ~ Binomial(CHAIN_N, CHAIN_P): fitted on 1,299 uma-it Independent Training runs whose chain
/// progress is readable from event hints (first on 879; a refit on 1,792 runs kept it) (mean 7.35, variance 4.8; no difference between scenarios; a Poisson count,
/// variance = mean, fits much worse). It slightly underestimates decks with two chain cards: both chains finish in
/// 82% of such runs, 75% predicted.
pub(super) const CHAIN_N: u32 = 21;
pub(super) const CHAIN_P: f64 = 0.35;
/// Chain kind of a deck entry: (chain length, link the chain stops at). A card played normally stops at its last
/// link; a card whose chain the player ends early (a choice that ends the chain) stops sooner. While it runs, an
/// entry is picked with weight = links it has left (length - done); once it reaches its stop it is out.
/// A deck entry's chain kind: (chain links, link it stops after, stop risk). Stop risk: links where IT's default
/// choice ends the chain on k of n outcomes, one byte per link 1..=4 ((k << 4) | n; 0 = none), only links before the stop.
pub(super) type Kind = (u8, u8, u32);
pub(super) fn stop_risk_code(risk: &[(u8, u8, u8)], stop: u8) -> u32 {
    risk.iter().filter(|r| r.0 >= 1 && r.0 < stop && r.0 <= 4 && r.2 > 0).fold(0u32, |c, r| c | ((((r.1 as u32) << 4) | r.2 as u32) << (8 * (r.0 as u32 - 1))))
}

thread_local! { pub(super) static CHAIN_EVENTS: std::cell::Cell<(u32, f64)> = std::cell::Cell::new((CHAIN_N, CHAIN_P)); }
/// P(N = e) chain events per run for e = 0..=n, N ~ Binomial of the job in progress (default CHAIN_N, CHAIN_P).
pub(super) fn event_pmf(n: usize) -> Vec<f64> {
    let (bn, bp) = CHAIN_EVENTS.with(|c| c.get());
    (0..=n).map(|e| if e as u32 <= bn { binom_coef(bn, e as u32) * bp.powi(e as i32) * (1.0 - bp).powi((bn - e as u32) as i32) } else { 0.0 }).collect()
}
/// `event_pmf` of the job's own event count (n = its CHAIN_EVENTS n), kept per (n, p): the same for every call of a job
pub(super) fn event_pmf_job() -> std::rc::Rc<Vec<f64>> {
    thread_local! { static EP: std::cell::RefCell<Option<((u32, u64), std::rc::Rc<Vec<f64>>)>> = const { std::cell::RefCell::new(None) }; }
    let key = pmf_key();
    if let Some(r) = EP.with(|c| c.borrow().as_ref().filter(|x| x.0 == key).map(|x| x.1.clone())) { return r; }
    let r = std::rc::Rc::new(event_pmf(key.0 as usize));
    EP.with(|c| *c.borrow_mut() = Some((key, r.clone())));
    r
}
pub(super) fn binom_coef(n: u32, k: u32) -> f64 { (0..k).fold(1.0f64, |a, i| a * (n - i) as f64 / (i + 1) as f64) }
/// Cache key of the event-count distribution in use: (n, bits of p). (An XOR of the two collided: (21, 0.35) and
/// (22, 0.7) gave one key.)
pub(super) fn pmf_key() -> (u32, u64) { CHAIN_EVENTS.with(|c| { let (n, p) = c.get(); (n, p.to_bits()) }) }
/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
/// Dense, deterministic, memoised version of `chain_marginals_ref`. States are mixed-radix indices over the
/// entries' progress; every transition raises the index, so one pass in index order is a topological order.
/// Entries are sorted first so the floating-point summation order is canonical (the HashMap reference sums in
/// iteration order, which varied from run to run in the last bits).
pub(super) fn chain_marginals(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    thread_local! { static CM: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), HashMap<Kind, Vec<f64>>>> = std::cell::RefCell::new(Default::default()); }
    let mut ks: Vec<Kind> = kinds.to_vec(); ks.sort();
    let key = (ks, pmf_key());
    if let Some(r) = CM.with(|c| c.borrow().get(&key).cloned()) { return r; }
    let r = if key.0.iter().any(|k| k.2 != 0) { chain_marginals_risk(&key.0) } else { chain_marginals_dense(&key.0) };
    CM.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}
/// Kinds with a stop risk: the chance that IT's default choice ends a chain at a link is independent of how many
/// chain events the run gets, so the run is a mixture over where each risky entry would stop. Each realisation is a
/// plain kind (links, realised stop): the entry competes for events with its full chain until it stops, then frees
/// its remaining slots, exactly as a chosen early end does. Exact.
pub(super) fn chain_marginals_risk(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    // per entry: [(realised stop, probability)]
    let alts: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(links, stop, risk)| {
        let mut v = vec![]; let mut go = 1.0f64;
        for l in 1..stop.min(5) {
            let b = (risk >> (8 * (l as u32 - 1))) & 0xff;
            if b == 0 { continue; }
            let q = (b >> 4) as f64 / (b & 15) as f64;
            v.push((l, go * q)); go *= 1.0 - q;
        }
        v.push((stop, go)); let _ = links; v
    }).collect();
    let n = kinds.len();
    let mut acc: Vec<Vec<f64>> = kinds.iter().map(|k| vec![0.0; k.1 as usize]).collect();
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let real: Vec<Kind> = (0..n).map(|i| (kinds[i].0, alts[i][pick[i]].0, 0)).collect();
            let m = chain_marginals(&real);
            for i in 0..n { if let Some(v) = m.get(&real[i]) { for (k, x) in v.iter().enumerate() { acc[i][k] += w * x; } } }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    for i in 0..n { out.entry(kinds[i]).or_insert_with(|| acc[i].clone()); }
    out
}
pub(super) fn chain_marginals_dense(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 { return out; }
    let emax = 60usize;
    let pois = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
    let last = emax.min(total);
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    let mut stride = vec![0usize; n]; let mut nst = 1usize;
    for i in 0..n { stride[i] = nst; nst *= kinds[i].1 as usize + 1; }
    let mut p = vec![0.0f64; nst]; p[0] = 1.0;
    // done[e][i][k] = P(after e events entry i has >= k+1 links done)
    let mut done = vec![0.0f64; (last + 1) * n * width];
    let mut s = vec![0u8; n]; let mut lvl = 0usize;
    for idx in 0..nst {
        let pi = p[idx];
        if pi != 0.0 && lvl <= last {
            if lvl >= 1 { let b = lvl * n * width; for i in 0..n { for k in 0..s[i] as usize { done[b + i * width + k] += pi; } } }
            if lvl < last {
                let mut wsum = 0.0f64;
                for i in 0..n { if s[i] < kinds[i].1 { wsum += (kinds[i].0 - s[i]) as f64; } }
                if wsum > 0.0 {
                    for i in 0..n { if s[i] < kinds[i].1 { let w = (kinds[i].0 - s[i]) as f64; p[idx + stride[i]] += pi * w / wsum; } }
                }
            }
        }
        // odometer increment of s, tracking the level
        for i in 0..n { if s[i] < kinds[i].1 { s[i] += 1; lvl += 1; break; } lvl -= s[i] as usize; s[i] = 0; }
    }
    let mut fired = vec![vec![0.0f64; width]; n];
    for e in 1..=last {
        let pe = pois[e] + if e == last { 1.0 - pois[..=e].iter().sum::<f64>() } else { 0.0 };
        let b = e * n * width;
        for i in 0..n { for k in 0..width { fired[i][k] += pe * done[b + i * width + k]; } }
    }
    for i in 0..n {
        let kd = kinds[i];
        out.entry(kd).or_insert_with(|| (0..kd.1 as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds
/// (reference implementation, for the tests).
#[cfg(test)]
pub(super) fn chain_marginals_ref(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 { return out; }
    let emax = 60usize;
    let pois = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    let mut fired = vec![vec![0.0f64; width]; n];      // P(entry i has >= k+1 links done), accumulated over E
    let mut st: HashMap<Vec<u8>, f64> = HashMap::new();
    st.insert(vec![0; n], 1.0);
    for e in 1..=emax.min(total) {
        let mut ns: HashMap<Vec<u8>, f64> = HashMap::new();
        for (s, p) in &st {
            let w: Vec<f64> = (0..n).map(|i| if s[i] < kinds[i].1 { (kinds[i].0 - s[i]) as f64 } else { 0.0 }).collect();
            let wsum: f64 = w.iter().sum();
            if wsum == 0.0 { *ns.entry(s.clone()).or_insert(0.0) += p; continue; }
            for i in 0..n { if w[i] > 0.0 { let mut t = s.clone(); t[i] += 1; *ns.entry(t).or_insert(0.0) += p * w[i] / wsum; } }
        }
        st = ns;
        let mut prev_done = vec![vec![0.0f64; width]; n];
        for (s, p) in &st { for i in 0..n { for k in 0..s[i] as usize { prev_done[i][k] += p; } } }
        // P(link done) = sum_e P(E = e) * P(done after e events); accumulate with P(E = e)
        let pe = pois[e] + if e == emax.min(total) { 1.0 - pois[..=e].iter().sum::<f64>() } else { 0.0 };
        for i in 0..n { for k in 0..width { fired[i][k] += pe * prev_done[i][k]; } }
    }
    for i in 0..n {
        let kd = kinds[i];
        out.entry(kd).or_insert_with(|| (0..kd.1 as usize).map(|k| fired[i][k]).collect());
    }
    out
}

/// Final-state distribution of the chain process behind `chain_marginals`: (links done per entry, probability).
/// Memoised per sorted kind list; `kinds` must be sorted. Exact: a state at level L (L events taken) is final
/// with P(N = L) unless every entry is at its stop, then with P(N >= L); stop-risk entries are the mixture over
/// where they stop, as in `chain_marginals_risk`.
pub(super) fn chain_states(kinds: &[Kind]) -> std::rc::Rc<Vec<(Vec<u8>, f64)>> {
    thread_local! { static CS: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), std::rc::Rc<Vec<(Vec<u8>, f64)>>>> = std::cell::RefCell::new(Default::default()); }
    let key = (kinds.to_vec(), pmf_key());
    if let Some(r) = CS.with(|c| c.borrow().get(&key).cloned()) { return r; }
    let n = kinds.len();
    let alts: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(_, stop, risk)| {
        let mut v = vec![]; let mut go = 1.0f64;
        for l in 1..stop.min(5) {
            let b = (risk >> (8 * (l as u32 - 1))) & 0xff;
            if b == 0 { continue; }
            let q = (b >> 4) as f64 / (b & 15) as f64;
            v.push((l, go * q)); go *= 1.0 - q;
        }
        v.push((stop, go)); v
    }).collect();
    let mut acc: crate::FxMap<Vec<u8>, f64> = Default::default();
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let stops: Vec<u8> = (0..n).map(|i| alts[i][pick[i]].0).collect();
            let total: usize = stops.iter().map(|&x| x as usize).sum();
            let pois = event_pmf(total);
            let mut stride = vec![0usize; n]; let mut nst = 1usize;
            for i in 0..n { stride[i] = nst; nst *= stops[i] as usize + 1; }
            let mut p = vec![0.0f64; nst]; p[0] = 1.0;
            let mut s = vec![0u8; n];
            for idx in 0..nst {
                let mut r = idx; let mut lvl = 0usize;
                for i in 0..n { s[i] = (r % (stops[i] as usize + 1)) as u8; r /= stops[i] as usize + 1; lvl += s[i] as usize; }
                let pi = p[idx];
                if pi == 0.0 { continue; }
                let mut wsum = 0.0f64;
                for i in 0..n { if s[i] < stops[i] { wsum += (kinds[i].0 - s[i]) as f64; } }
                let fin = if wsum > 0.0 {
                    for i in 0..n { if s[i] < stops[i] { p[idx + stride[i]] += pi * (kinds[i].0 - s[i]) as f64 / wsum; } }
                    pois[lvl]
                } else { (1.0 - pois[..lvl].iter().sum::<f64>()).max(0.0) };
                *acc.entry(s.clone()).or_insert(0.0) += w * pi * fin;
            }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut v: Vec<(Vec<u8>, f64)> = acc.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    let r = std::rc::Rc::new(v);
    CS.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}

/// Per chain-kind key (sorted kinds, fillers as (2, 2)): for each position, P(links done = l | N = n), n = 0..=nmax,
/// and whether the progress vector given every n is negatively upper-orthant dependent:
///   P(X_i >= t_i for all i) <= prod_i P(X_i >= t_i)   for every threshold vector t,
/// checked exactly (to 1e-12) over the key's state distribution. Early ends and stop risk are included (stop risk as
/// the mixture over where each risky card stops, which does not depend on N).
pub(super) struct KeyLevel { pub(super) marg: Vec<Vec<Vec<f64>>>, pub(super) nuod: bool, pub(super) lev: Vec<Vec<f64>>, pub(super) dims: Vec<usize>, pub(super) gstride: Vec<usize>,
                  pub(super) pairs: std::cell::RefCell<crate::FxMap<(usize, usize), bool>>,
                  pub(super) pms: std::cell::RefCell<crate::FxMap<(usize, usize), std::rc::Rc<Vec<Vec<Vec<f64>>>>>> }
impl KeyLevel {
    /// P(X_i = a, X_j = b | N = m)
    pub(super) fn pair_marg(&self, i: usize, j: usize, m: usize) -> Vec<Vec<f64>> {
        let mut v = vec![vec![0.0f64; self.dims[j]]; self.dims[i]];
        for (gi, &p) in self.lev[m].iter().enumerate() { if p != 0.0 { v[(gi / self.gstride[i]) % self.dims[i]][(gi / self.gstride[j]) % self.dims[j]] += p; } }
        v
    }
    /// `pair_marg` at every event count m = 0..=n, kept per pair (a function of the key alone)
    pub(super) fn pair_margs(&self, i: usize, j: usize) -> std::rc::Rc<Vec<Vec<Vec<f64>>>> {
        if let Some(r) = self.pms.borrow().get(&(i, j)) { return r.clone(); }
        let r = std::rc::Rc::new((0..self.lev.len()).map(|m| self.pair_marg(i, j, m)).collect::<Vec<_>>());
        self.pms.borrow_mut().insert((i, j), r.clone());
        r
    }
    /// The orthant check with positions i and j as one block: for every up-set A of the (i, j) grid and every
    /// threshold vector t on the other positions, P((X_i, X_j) in A, X_k >= t_k) <= P((X_i, X_j) in A) prod P(X_k >= t_k).
    /// Any rising function of (X_i, X_j) is a positive combination of up-set indicators, so this is what the bound
    /// needs when one group holds both cards.
    pub(super) fn pair_ok(&self, i: usize, j: usize) -> bool {
        if let Some(&r) = self.pairs.borrow().get(&(i, j)) { return r; }
        let (di, dj) = (self.dims[i], self.dims[j]);
        // up-sets of the di x dj grid: for each row a, the first column c_a in the set, non-increasing in a
        let mut ups: Vec<Vec<usize>> = vec![];
        fn upsets(a: usize, di: usize, dj: usize, hi: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            if a == di { out.push(cur.clone()); return; }
            for c in 0..=hi { cur.push(c); upsets(a + 1, di, dj, c, cur, out); cur.pop(); }
        }
        upsets(0, di, dj, dj, &mut vec![], &mut ups);
        let n = self.dims.len();
        let others: Vec<usize> = (0..n).filter(|&k| k != i && k != j).collect();
        let mut ok = true;
        'outer: for m in 0..self.lev.len() {
            let d = &self.lev[m];
            for up in &ups {
                // indicator of the up-set (column c >= up[row]) over the grid, times the distribution
                let mut arr: Vec<f64> = d.iter().enumerate().map(|(gi, &p)| { let a = (gi / self.gstride[i]) % di; let c = (gi / self.gstride[j]) % dj; if c >= up[a] { p } else { 0.0 } }).collect();
                let pa: f64 = arr.iter().sum();
                for &k in &others {
                    for gi in (0..arr.len()).rev() { let c = (gi / self.gstride[k]) % self.dims[k]; if c + 1 < self.dims[k] { arr[gi] += arr[gi + self.gstride[k]]; } }
                }
                for gi in 0..arr.len() {
                    if (gi / self.gstride[i]) % di != 0 || (gi / self.gstride[j]) % dj != 0 { continue; }
                    let prod: f64 = others.iter().map(|&k| self.marg[k][m][(gi / self.gstride[k]) % self.dims[k]..].iter().sum::<f64>()).product();
                    if arr[gi] > pa * prod * (1.0 + 1e-12) + 1e-15 { ok = false; break 'outer; }
                }
            }
        }
        self.pairs.borrow_mut().insert((i, j), ok);
        ok
    }
}
pub(super) fn key_level(kinds: &[Kind]) -> std::rc::Rc<KeyLevel> {
    thread_local! { static KL: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), std::rc::Rc<KeyLevel>>> = std::cell::RefCell::new(Default::default()); }
    thread_local! { static KL_LAST: std::cell::RefCell<Option<(Vec<Kind>, (u32, u64), std::rc::Rc<KeyLevel>)>> = const { std::cell::RefCell::new(None) }; }
    let pk = pmf_key();
    if let Some(r) = KL_LAST.with(|c| c.borrow().as_ref().filter(|x| x.0 == kinds && x.1 == pk).map(|x| x.2.clone())) { return r; }
    let key = (kinds.to_vec(), pk);
    if let Some(r) = KL.with(|c| c.borrow().get(&key).cloned()) { KL_LAST.with(|c| *c.borrow_mut() = Some((key.0, key.1, r.clone()))); return r; }
    let (bn, _) = CHAIN_EVENTS.with(|c| c.get());
    let nmax = bn as usize;
    let n = kinds.len();
    let dims: Vec<usize> = kinds.iter().map(|k| k.1 as usize + 1).collect();
    let mut gstride = vec![1usize; n]; let mut gsize = 1usize;
    for i in 0..n { gstride[i] = gsize; gsize *= dims[i]; }
    // per level, the distribution over the grid of progress vectors
    let mut lev = vec![vec![0.0f64; gsize]; nmax + 1];
    let alts: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(_, stop, risk)| {
        let mut v = vec![]; let mut go = 1.0f64;
        for l in 1..stop.min(5) {
            let b = (risk >> (8 * (l as u32 - 1))) & 0xff;
            if b == 0 { continue; }
            let q = (b >> 4) as f64 / (b & 15) as f64;
            v.push((l, go * q)); go *= 1.0 - q;
        }
        v.push((stop, go)); v
    }).collect();
    let mut pick = vec![0usize; n];
    loop {
        let w: f64 = (0..n).map(|i| alts[i][pick[i]].1).product();
        if w > 0.0 {
            let stops: Vec<u8> = (0..n).map(|i| alts[i][pick[i]].0).collect();
            let total: usize = stops.iter().map(|&x| x as usize).sum();
            let mut stride = vec![0usize; n]; let mut nst = 1usize;
            for i in 0..n { stride[i] = nst; nst *= stops[i] as usize + 1; }
            let mut p = vec![0.0f64; nst]; p[0] = 1.0;
            let mut st = vec![0u8; n];
            for idx in 0..nst {
                let mut r = idx; let mut lvl = 0usize; let mut gi = 0usize;
                for i in 0..n { st[i] = (r % (stops[i] as usize + 1)) as u8; r /= stops[i] as usize + 1; lvl += st[i] as usize; gi += st[i] as usize * gstride[i]; }
                let pi = p[idx];
                if pi == 0.0 { continue; }
                let mut wsum = 0.0f64;
                for i in 0..n { if st[i] < stops[i] { wsum += (kinds[i].0 - st[i]) as f64; } }
                if wsum > 0.0 { for i in 0..n { if st[i] < stops[i] { p[idx + stride[i]] += pi * (kinds[i].0 - st[i]) as f64 / wsum; } } }
                if lvl <= nmax { lev[lvl][gi] += w * pi; }
                if lvl == total { for m in total + 1..=nmax { lev[m][gi] += w * pi; } }
            }
        }
        let mut i = 0;
        while i < n { pick[i] += 1; if pick[i] < alts[i].len() { break; } pick[i] = 0; i += 1; }
        if i == n { break; }
    }
    let mut marg = vec![vec![vec![0.0f64; 0]; nmax + 1]; n];
    let mut nuod = true;
    for m in 0..=nmax {
        let d = &lev[m];
        for i in 0..n {
            let mut v = vec![0.0f64; dims[i]];
            for gi in 0..gsize { v[(gi / gstride[i]) % dims[i]] += d[gi]; }
            marg[i][m] = v;
        }
        if !nuod { continue; }
        // upper-orthant sums by suffix sums along every axis
        let mut up = d.clone();
        for i in 0..n {
            for gi in (0..gsize).rev() { let c = (gi / gstride[i]) % dims[i]; if c + 1 < dims[i] { up[gi] += up[gi + gstride[i]]; } }
        }
        let tail: Vec<Vec<f64>> = (0..n).map(|i| { let mut t = marg[i][m].clone(); for c in (0..dims[i] - 1).rev() { t[c] += t[c + 1]; } t }).collect();
        for gi in 0..gsize {
            let prod: f64 = (0..n).map(|i| tail[i][(gi / gstride[i]) % dims[i]]).product();
            if up[gi] > prod * (1.0 + 1e-12) + 1e-15 { nuod = false; break; }
        }
    }
    let r = std::rc::Rc::new(KeyLevel { marg, nuod, lev, dims, gstride, pairs: Default::default(), pms: Default::default() });
    KL_LAST.with(|c| *c.borrow_mut() = Some((key.0.clone(), key.1, r.clone())));
    KL.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}
