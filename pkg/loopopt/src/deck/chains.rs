//! Chain events: the event count per run, the per-link marginals and the final-state distribution of a deck's chains,
//! and the per-key level tables.
use super::HashMap;

/// Chain events per run ~ Binomial(CHAIN_N, CHAIN_P): fitted on 1,299 uma-it Independent Training runs whose chain
/// progress is readable from event hints (an earlier fit on 879 runs and a refit on 1,792, with markers read from IT's
/// default choice, chose the same) (mean 7.35, variance 4.8; no difference between scenarios; a Poisson count,
/// variance = mean, fits much worse). Decks with two chain cards finish both chains in 83% of such runs, 80% predicted.
pub(super) const CHAIN_N: u32 = 21;
pub(super) const CHAIN_P: f64 = 0.35;
/// A deck entry's chain kind: (chain links, link it stops after, stop risk). A card played normally stops at its last
/// link; a chosen early end stops sooner. While it runs, an entry is picked with weight = links it has left
/// (length - done); once it reaches its stop it is out. Stop risk: links where IT's default
/// choice ends the chain on k of n outcomes, one byte per link 1..=4 ((k << 4) | n; 0 = none), only links before the stop.
pub(super) type Kind = (u8, u8, u32);
pub(super) fn stop_risk_code(risk: &[(u8, u8, u8)], stop: u8) -> u32 {
    risk.iter()
        .filter(|r| r.0 >= 1 && r.0 < stop && r.0 <= 4 && r.2 > 0)
        .fold(0u32, |c, r| c | (((u32::from(r.1) << 4) | u32::from(r.2)) << (8 * (u32::from(r.0) - 1))))
}

thread_local! { pub(super) static CHAIN_EVENTS: std::cell::Cell<(u32, f64)> = const { std::cell::Cell::new((CHAIN_N, CHAIN_P)) }; }
/// P(N = e) chain events per run for e = 0..=n, N ~ Binomial of the job in progress (default CHAIN_N, CHAIN_P).
pub(super) fn event_pmf(n: usize) -> Vec<f64> {
    let (bn, bp) = CHAIN_EVENTS.with(std::cell::Cell::get);
    (0..=n)
        .map(|e| {
            if e as u32 <= bn {
                binom_coef(bn, e as u32) * bp.powi(e as i32) * (1.0 - bp).powi((bn - e as u32) as i32)
            } else {
                0.0
            }
        })
        .collect()
}
/// `event_pmf` of the job's own event count (n = its CHAIN_EVENTS n), kept per (n, p): the same for every call of a job
pub(super) fn event_pmf_job() -> std::rc::Rc<Vec<f64>> {
    thread_local! { static EP: std::cell::RefCell<Option<((u32, u64), std::rc::Rc<Vec<f64>>)>> = const { std::cell::RefCell::new(None) }; }
    let key = pmf_key();
    if let Some(r) = EP.with(|c| c.borrow().as_ref().filter(|x| x.0 == key).map(|x| x.1.clone())) {
        return r;
    }
    let r = std::rc::Rc::new(event_pmf(key.0 as usize));
    EP.with(|c| *c.borrow_mut() = Some((key, r.clone())));
    r
}
pub(super) fn binom_coef(n: u32, k: u32) -> f64 {
    (0..k).fold(1.0f64, |a, i| a * f64::from(n - i) / f64::from(i + 1))
}
/// Cache key of the event-count distribution in use: (n, bits of p). (An XOR of the two collided: (21, 0.35) and
/// (22, 0.7) gave one key.)
pub(super) fn pmf_key() -> (u32, u64) {
    CHAIN_EVENTS.with(|c| {
        let (n, p) = c.get();
        (n, p.to_bits())
    })
}
/// Where an entry of stop risk `risk` (`Kind`) and planned stop `stop` stops: [(realised stop, probability)], the risky
/// links in order then the planned stop with what is left. The one decoding of `Kind`'s risk bytes
fn stop_alts(stop: u8, risk: u32) -> Vec<(u8, f64)> {
    let mut alternatives = vec![];
    let mut surviving_probability = 1.0f64;
    for link in 1..stop.min(5) {
        let risk_byte = (risk >> (8 * (u32::from(link) - 1))) & 0xff;
        if risk_byte == 0 {
            continue;
        }
        let stop_probability = f64::from(risk_byte >> 4) / f64::from(risk_byte & 15);
        alternatives.push((link, surviving_probability * stop_probability));
        surviving_probability *= 1.0 - stop_probability;
    }
    alternatives.push((stop, surviving_probability));
    alternatives
}
/// The stop-risk mixture: every realisation of where the entries stop (one alternative of `stop_alts` each, the first
/// entry's choice varying fastest), with its probability w (the product in entry order), `visit(w, stops)` where w > 0.
fn for_each_realisation(kinds: &[Kind], mut visit: impl FnMut(f64, &[u8])) {
    let alternatives: Vec<Vec<(u8, f64)>> = kinds.iter().map(|&(_, stop, risk)| stop_alts(stop, risk)).collect();
    let n = kinds.len();
    let mut choices = vec![0usize; n];
    let mut stops = vec![0u8; n];
    loop {
        let probability: f64 = (0..n).map(|i| alternatives[i][choices[i]].1).product();
        if probability > 0.0 {
            for i in 0..n {
                stops[i] = alternatives[i][choices[i]].0;
            }
            visit(probability, &stops);
        }
        // Advance the mixed-radix choice vector, first entry fastest. With no entries, visit once.
        let mut i = 0;
        while i < n {
            choices[i] += 1;
            if choices[i] < alternatives[i].len() {
                break;
            }
            choices[i] = 0;
            i += 1;
        }
        if i == n {
            break;
        }
    }
}
/// The chain process for entries of `links` links stopping at `stops` (no risk left). States are in mixed-radix
/// index order: entry i's progress is the digit with stride prod_{j<i} (stops[j] + 1). Every transition raises the
/// index, so this order is topological. `visit(progress, events, grid_index, probability)` receives reachable states
/// up to `max_events`; `grid_strides` maps their progress to the caller's grid. Below `max_events`, an entry takes
/// the next event with weight its links left over the sum among entries not stopped.
/// The one transition rule of `chain_marginals_dense`, `chain_states` and `key_level`.
#[inline(always)]
fn chain_walk(
    links: &[u8],
    stops: &[u8],
    max_events: usize,
    grid_strides: &[usize],
    mut visit: impl FnMut(&[u8], usize, usize, f64),
) {
    let n = links.len();
    let mut strides = vec![0usize; n];
    let mut state_count = 1usize;
    for i in 0..n {
        strides[i] = state_count;
        state_count *= stops[i] as usize + 1;
    }
    let mut probabilities = vec![0.0f64; state_count];
    probabilities[0] = 1.0;
    // Advance progress, its sum and its caller-grid index together, as an odometer.
    let mut progress = vec![0u8; n];
    let (mut events, mut grid_index) = (0usize, 0usize);
    for state_index in 0..state_count {
        let probability = probabilities[state_index];
        if probability != 0.0 && events <= max_events {
            visit(&progress, events, grid_index, probability);
            if events < max_events {
                let mut total_weight = 0.0f64;
                for i in 0..n {
                    if progress[i] < stops[i] {
                        total_weight += f64::from(links[i] - progress[i]);
                    }
                }
                if total_weight > 0.0 {
                    for i in 0..n {
                        if progress[i] < stops[i] {
                            probabilities[state_index + strides[i]] +=
                                probability * f64::from(links[i] - progress[i]) / total_weight;
                        }
                    }
                }
            }
        }
        for i in 0..n {
            if progress[i] < stops[i] {
                progress[i] += 1;
                events += 1;
                grid_index += grid_strides[i];
                break;
            }
            events -= progress[i] as usize;
            grid_index -= progress[i] as usize * grid_strides[i];
            progress[i] = 0;
        }
    }
}
/// P(link k of an entry fires), k = 1..=stop, for every chain kind present, given the deck's chain kinds.
/// Dense, deterministic, memoised version of `chain_marginals_ref`. States are mixed-radix indices over the
/// entries' progress; every transition raises the index, so one pass in index order is a topological order.
/// Entries are sorted first so the floating-point summation order is canonical (the HashMap reference sums in
/// iteration order, which varied from run to run in the last bits).
pub(super) fn chain_marginals(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    thread_local! { static CM: std::cell::RefCell<crate::FxMap<(Vec<Kind>, (u32, u64)), HashMap<Kind, Vec<f64>>>> = std::cell::RefCell::new(Default::default()); }
    let mut ks: Vec<Kind> = kinds.to_vec();
    ks.sort_unstable();
    let key = (ks, pmf_key());
    if let Some(r) = CM.with(|c| c.borrow().get(&key).cloned()) {
        return r;
    }
    let r = if key.0.iter().any(|k| k.2 != 0) { chain_marginals_risk(&key.0) } else { chain_marginals_dense(&key.0) };
    CM.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}
/// Kinds with a stop risk: the chance that IT's default choice ends a chain at a link is independent of how many
/// chain events the run gets, so the run is a mixture over where each risky entry would stop. Each realisation is a
/// plain kind (links, realised stop): the entry competes for events with its full chain until it stops, then frees
/// its remaining slots, exactly as a chosen early end does. Exact.
pub(super) fn chain_marginals_risk(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut acc: Vec<Vec<f64>> = kinds.iter().map(|k| vec![0.0; k.1 as usize]).collect();
    for_each_realisation(kinds, |w, stops| {
        let real: Vec<Kind> = (0..n).map(|i| (kinds[i].0, stops[i], 0)).collect();
        let m = chain_marginals(&real);
        for i in 0..n {
            if let Some(v) = m.get(&real[i]) {
                for (k, x) in v.iter().enumerate() {
                    acc[i][k] += w * x;
                }
            }
        }
    });
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    for i in 0..n {
        out.entry(kinds[i]).or_insert_with(|| acc[i].clone());
    }
    out
}
pub(super) fn chain_marginals_dense(kinds: &[Kind]) -> HashMap<Kind, Vec<f64>> {
    let n = kinds.len();
    let mut out: HashMap<Kind, Vec<f64>> = HashMap::new();
    if n == 0 {
        return out;
    }
    let emax = 60usize;
    let event_weights = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
    let last = emax.min(total);
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    // done[e][i][k] = P(after e events entry i has >= k+1 links done)
    let mut done = vec![0.0f64; (last + 1) * n * width];
    let (links, stops): (Vec<u8>, Vec<u8>) = kinds.iter().map(|k| (k.0, k.1)).unzip();
    chain_walk(&links, &stops, last, &vec![0; n], |s, lvl, _, pi| {
        if lvl >= 1 {
            let b = lvl * n * width;
            for i in 0..n {
                for k in 0..s[i] as usize {
                    done[b + i * width + k] += pi;
                }
            }
        }
    });
    let mut fired = vec![vec![0.0f64; width]; n];
    for e in 1..=last {
        let pe = event_weights[e] + if e == last { 1.0 - event_weights[..=e].iter().sum::<f64>() } else { 0.0 };
        let b = e * n * width;
        for i in 0..n {
            for k in 0..width {
                fired[i][k] += pe * done[b + i * width + k];
            }
        }
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
    if n == 0 {
        return out;
    }
    let emax = 60usize;
    let event_weights = event_pmf(emax);
    let total: usize = kinds.iter().map(|&(_, stop, _)| stop as usize).sum();
    let width = kinds.iter().map(|k| k.1 as usize).max().unwrap_or(0).max(1);
    let mut fired = vec![vec![0.0f64; width]; n]; // P(entry i has >= k+1 links done), accumulated over E
    let mut st: HashMap<Vec<u8>, f64> = HashMap::new();
    st.insert(vec![0; n], 1.0);
    for e in 1..=emax.min(total) {
        let mut ns: HashMap<Vec<u8>, f64> = HashMap::new();
        for (s, p) in &st {
            let w: Vec<f64> =
                (0..n).map(|i| if s[i] < kinds[i].1 { f64::from(kinds[i].0 - s[i]) } else { 0.0 }).collect();
            let wsum: f64 = w.iter().sum();
            if wsum == 0.0 {
                *ns.entry(s.clone()).or_insert(0.0) += p;
                continue;
            }
            for i in 0..n {
                if w[i] > 0.0 {
                    let mut t = s.clone();
                    t[i] += 1;
                    *ns.entry(t).or_insert(0.0) += p * w[i] / wsum;
                }
            }
        }
        st = ns;
        let mut prev_done = vec![vec![0.0f64; width]; n];
        for (s, p) in &st {
            for i in 0..n {
                for k in 0..s[i] as usize {
                    prev_done[i][k] += p;
                }
            }
        }
        // P(link done) = sum_e P(E = e) * P(done after e events); accumulate with P(E = e)
        let pe =
            event_weights[e] + if e == emax.min(total) { 1.0 - event_weights[..=e].iter().sum::<f64>() } else { 0.0 };
        for i in 0..n {
            for k in 0..width {
                fired[i][k] += pe * prev_done[i][k];
            }
        }
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
    if let Some(r) = CS.with(|c| c.borrow().get(&key).cloned()) {
        return r;
    }
    let n = kinds.len();
    let links: Vec<u8> = kinds.iter().map(|k| k.0).collect();
    let mut acc: crate::FxMap<Vec<u8>, f64> = Default::default();
    for_each_realisation(kinds, |w, stops| {
        let total: usize = stops.iter().map(|&x| x as usize).sum();
        let event_weights = event_pmf(total);
        // (a state below the last level has an entry not stopped: final with P(N = L); at it, with P(N >= L))
        chain_walk(&links, stops, total, &vec![0; n], |s, lvl, _, pi| {
            let fin = if lvl < total {
                event_weights[lvl]
            } else {
                (1.0 - event_weights[..lvl].iter().sum::<f64>()).max(0.0)
            };
            *acc.entry(s.to_vec()).or_insert(0.0) += w * pi * fin;
        });
    });
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
pub(super) struct KeyLevel {
    pub(super) marg: Vec<Vec<Vec<f64>>>,
    pub(super) nuod: bool,
    pub(super) lev: Vec<Vec<f64>>,
    pub(super) dims: Vec<usize>,
    pub(super) gstride: Vec<usize>,
    pub(super) pairs: std::cell::RefCell<crate::FxMap<(usize, usize), bool>>,
    pub(super) pms: std::cell::RefCell<crate::FxMap<(usize, usize), std::rc::Rc<Vec<Vec<Vec<f64>>>>>>,
}
impl KeyLevel {
    /// P(X_i = a, X_j = b | N = m)
    pub(super) fn pair_marg(&self, i: usize, j: usize, m: usize) -> Vec<Vec<f64>> {
        let mut v = vec![vec![0.0f64; self.dims[j]]; self.dims[i]];
        for (gi, &p) in self.lev[m].iter().enumerate() {
            if p != 0.0 {
                v[(gi / self.gstride[i]) % self.dims[i]][(gi / self.gstride[j]) % self.dims[j]] += p;
            }
        }
        v
    }
    /// `pair_marg` at every event count m = 0..=n, kept per pair (a function of the key alone)
    pub(super) fn pair_margs(&self, i: usize, j: usize) -> std::rc::Rc<Vec<Vec<Vec<f64>>>> {
        if let Some(r) = self.pms.borrow().get(&(i, j)) {
            return r.clone();
        }
        let r = std::rc::Rc::new((0..self.lev.len()).map(|m| self.pair_marg(i, j, m)).collect::<Vec<_>>());
        self.pms.borrow_mut().insert((i, j), r.clone());
        r
    }
    /// The orthant check with positions i and j as one block: for every up-set A of the (i, j) grid and every
    /// threshold vector t on the other positions, P((X_i, X_j) in A, X_k >= t_k) <= P((X_i, X_j) in A) prod P(X_k >= t_k).
    /// Any rising function of (X_i, X_j) is a positive combination of up-set indicators, so this is what the bound
    /// needs when one group holds both cards.
    pub(super) fn pair_ok(&self, i: usize, j: usize) -> bool {
        if let Some(&r) = self.pairs.borrow().get(&(i, j)) {
            return r;
        }
        let (di, dj) = (self.dims[i], self.dims[j]);
        // up-sets of the di x dj grid: for each row a, the first column c_a in the set, non-increasing in a
        let mut ups: Vec<Vec<usize>> = vec![];
        fn upsets(a: usize, di: usize, hi: usize, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
            if a == di {
                out.push(cur.clone());
                return;
            }
            for c in 0..=hi {
                cur.push(c);
                upsets(a + 1, di, c, cur, out);
                cur.pop();
            }
        }
        upsets(0, di, dj, &mut vec![], &mut ups);
        let n = self.dims.len();
        let others: Vec<usize> = (0..n).filter(|&k| k != i && k != j).collect();
        let mut ok = true;
        'outer: for m in 0..self.lev.len() {
            let d = &self.lev[m];
            for up in &ups {
                // indicator of the up-set (column c >= up[row]) over the grid, times the distribution
                let mut arr: Vec<f64> = d
                    .iter()
                    .enumerate()
                    .map(|(gi, &p)| {
                        let a = (gi / self.gstride[i]) % di;
                        let c = (gi / self.gstride[j]) % dj;
                        if c >= up[a] { p } else { 0.0 }
                    })
                    .collect();
                let pa: f64 = arr.iter().sum();
                for &k in &others {
                    for gi in (0..arr.len()).rev() {
                        let c = (gi / self.gstride[k]) % self.dims[k];
                        if c + 1 < self.dims[k] {
                            arr[gi] += arr[gi + self.gstride[k]];
                        }
                    }
                }
                for gi in 0..arr.len() {
                    if !(gi / self.gstride[i]).is_multiple_of(di) || !(gi / self.gstride[j]).is_multiple_of(dj) {
                        continue;
                    }
                    let prod: f64 = others
                        .iter()
                        .map(|&k| self.marg[k][m][(gi / self.gstride[k]) % self.dims[k]..].iter().sum::<f64>())
                        .product();
                    if arr[gi] > pa * prod * (1.0 + 1e-12) + 1e-15 {
                        ok = false;
                        break 'outer;
                    }
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
    if let Some(r) = KL_LAST.with(|c| c.borrow().as_ref().filter(|x| x.0 == kinds && x.1 == pk).map(|x| x.2.clone())) {
        return r;
    }
    let key = (kinds.to_vec(), pk);
    if let Some(r) = KL.with(|c| c.borrow().get(&key).cloned()) {
        KL_LAST.with(|c| *c.borrow_mut() = Some((key.0, key.1, r.clone())));
        return r;
    }
    let (bn, _) = CHAIN_EVENTS.with(std::cell::Cell::get);
    let nmax = bn as usize;
    let n = kinds.len();
    let dims: Vec<usize> = kinds.iter().map(|k| k.1 as usize + 1).collect();
    let mut gstride = vec![1usize; n];
    let mut gsize = 1usize;
    for i in 0..n {
        gstride[i] = gsize;
        gsize *= dims[i];
    }
    // per level, the distribution over the grid of progress vectors
    let mut lev = vec![vec![0.0f64; gsize]; nmax + 1];
    let links: Vec<u8> = kinds.iter().map(|k| k.0).collect();
    for_each_realisation(kinds, |w, stops| {
        let total: usize = stops.iter().map(|&x| x as usize).sum();
        chain_walk(&links, stops, total, &gstride, |_, lvl, gi, pi| {
            if lvl <= nmax {
                lev[lvl][gi] += w * pi;
            }
            if lvl == total {
                for m in total + 1..=nmax {
                    lev[m][gi] += w * pi;
                }
            }
        });
    });
    let mut marg = vec![vec![vec![0.0f64; 0]; nmax + 1]; n];
    let mut nuod = true;
    // each grid index's digits, dig[gi * n + i] = (gi / gstride[i]) % dims[i] (the same sums below, in the same order)
    let mut dig = vec![0u8; gsize * n];
    {
        let mut st = vec![0u8; n];
        for gi in 0..gsize {
            dig[gi * n..(gi + 1) * n].copy_from_slice(&st);
            for i in 0..n {
                if (st[i] as usize) + 1 < dims[i] {
                    st[i] += 1;
                    break;
                }
                st[i] = 0;
            }
        }
    }
    for m in 0..=nmax {
        let d = &lev[m];
        // (grid index gi = b + c * gstride[i] + lo: by blocks b of gstride[i] * dims[i], then digit c, then lo, the
        // indices in increasing order, so each sum takes its terms in the same order)
        for i in 0..n {
            let (sd, dm) = (gstride[i], dims[i]);
            let mut v = vec![0.0f64; dm];
            for blk in d.chunks_exact(sd * dm) {
                for (c, row) in blk.chunks_exact(sd).enumerate() {
                    for &x in row {
                        v[c] += x;
                    }
                }
            }
            marg[i][m] = v;
        }
        if !nuod {
            continue;
        }
        // upper-orthant sums by suffix sums along every axis (digit c from the top down: each entry adds the one a
        // digit above it, already summed; the entries of one digit are independent)
        let mut up = d.clone();
        for i in 0..n {
            let (sd, dm) = (gstride[i], dims[i]);
            for blk in up.chunks_exact_mut(sd * dm) {
                for c in (0..dm - 1).rev() {
                    let (lo, hi) = blk[c * sd..(c + 2) * sd].split_at_mut(sd);
                    for (x, &y) in lo.iter_mut().zip(hi.iter()) {
                        *x += y;
                    }
                }
            }
        }
        let tail: Vec<Vec<f64>> = (0..n)
            .map(|i| {
                let mut t = marg[i][m].clone();
                for c in (0..dims[i] - 1).rev() {
                    t[c] += t[c + 1];
                }
                t
            })
            .collect();
        for gi in 0..gsize {
            let dg = &dig[gi * n..(gi + 1) * n];
            let prod: f64 = (0..n).map(|i| tail[i][dg[i] as usize]).product();
            if up[gi] > prod * (1.0 + 1e-12) + 1e-15 {
                nuod = false;
                break;
            }
        }
    }
    let r = std::rc::Rc::new(KeyLevel {
        marg,
        nuod,
        lev,
        dims,
        gstride,
        pairs: Default::default(),
        pms: Default::default(),
    });
    KL_LAST.with(|c| *c.borrow_mut() = Some((key.0.clone(), key.1, r.clone())));
    KL.with(|c| c.borrow_mut().insert(key, r.clone()));
    r
}
