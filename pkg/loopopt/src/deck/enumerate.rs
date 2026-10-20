//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
use super::*;

impl<'a> DeckSpace<'a> {
    /// Decks legal for trainees `us` (uma indices; `ids` their ids). `clauses`: ((trainee position in `us`,
    /// inheritance chance) pairs, threshold); a deck's clause value is the sum of its product values (fixed factor
    /// included) over those pairs. `best`: return the one deck
    /// with the highest value of the single clause above `seed` (none if no deck beats it; ties: the first found).
    /// Otherwise return every deck
    /// for which some clause has c * value >= threshold, c the deck's class cap (`cap_class`, `caps`) at most rho; at
    /// most `cap` decks (the flag says the cap was hit).
    pub(super) fn search(&self, model: &Model, us: &[usize], ids: &[u32], clauses: &[(Vec<(usize, f64)>, f64)], best: bool, seed: f64, rho: f64, cap: usize) -> (Vec<(f64, Sel)>, bool) {
        let nu = self.unc.len(); let size = self.dj.deck_size;
        if self.locked.iter().any(|&l| ids.iter().any(|&id| self.cards[l].blocks(id))) { return (vec![], false); }
        let (legal_rc, set_id) = self.reduced_cached(model, us, ids);
        let lt = self.legal_tabs(&legal_rc, set_id, ids);
        if lt.over { return (vec![], true); }
        let nplain = lt.nplain;
        let ebit: &[u128] = &lt.ebit;
        let nk = self.kinds_all.len();
        let ki22 = self.kinds_all.iter().position(|&k| k == (2, 2, 0)).unwrap();
        let kpos: Vec<u64> = (0..nk).map(|i| 7u64.pow(i as u32)).collect();
        assert!(nk < 64);
        let (by_kind, pal_ents) = (&lt.by_kind, &lt.pal_ents);
        let fixed = model.fixed_factor();
        // the state-conditioned node bound (SBOUND; the career's band pass, one trainee): per legal entry its miss rows
        // per chain level; off when an entry has an outcome hinting two targets (`state_value`)
        let sbm = if !best && clauses.len() == 1 && clauses[0].0.len() == 1 && SB_ON.with(|c| c.get()) { sb_mode() } else { 0 };
        let sb_rows: std::rc::Rc<Vec<Option<Vec<(f64, f64)>>>> = if sbm > 0 && !legal_rc.iter().any(|&j| self.srcs[j].multi) {
            let hit = self.sbrows.borrow().get(&set_id).cloned();
            hit.unwrap_or_else(|| { let r = std::rc::Rc::new((0..self.rel.len()).map(|j| if legal_rc.contains(&j) { Some(src_levels(&self.srcs[j], self.kind[j].1, nu)) } else { None }).collect::<Vec<_>>());
                self.sbrows.borrow_mut().insert(set_id, r.clone()); r })
        } else { std::rc::Rc::new(vec![]) };
        let sbm = if sb_rows.is_empty() { 0 } else { sbm };
        let sb_bar = if sbm > 0 { clauses[0].1 } else { 0.0 };
        let (sb_u, sb_q) = if sbm > 0 { (us[clauses[0].0[0].0], clauses[0].0[0].1) } else { (0, 0.0) };
        let rho = if sbm == 2 { 1e9 } else { rho };
        if sbm > 0 { SB_USED.with(|c| c.set(sbm)); }
        // the cap per structure class (`cap_class`), at most rho (uncapped with SBOUND=2); a deck passes at its own
        // class's cap, a node at the cap of the class of every deck it can still reach (its cards and every candidate
        // left: a superset of any completion's, so a cap at least as high)
        let ccap: [f64; 4] = if sbm == 2 { [rho; 4] } else { let c = self.caps(); [c[0].min(rho), c[1].min(rho), c[2].min(rho), c[3].min(rho)] };
        let val = |cl: &(Vec<(usize, f64)>, f64), dw: &[f64], dg: &[f64]| fixed * cl.0.iter().map(|&(k, q)| model.trainee_fast(us[k], q, dw, dg)).sum::<f64>();
        // every chance and outfit odd in [0, 1] and a fixed factor >= 0: then a deck's value rises with each of its odds
        // (white < gold) and every quantity in its evaluation is in [0, 1] up to rounding, which the dominance skip
        // (`dom`), the lazy configurations (`Pend`) and the key bound (`keyskip`) rely on
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let valid = fixed >= 0.0 && clauses.iter().all(|cl| cl.0.iter().all(|&(k, q)| in01(q) && model.ews[us[k]].iter().flatten().all(|&x| in01(x))));
        let dom_on = best && valid;
        // the product search builds its configurations lazily (`Pend`): roots and ratio tables only for those it may
        // visit
        let lazy = best && valid;
        let t_cfg = Instant::now();
        let mut out: Vec<(f64, Sel)> = vec![];
        let mut bestv = if best { seed } else { f64::NEG_INFINITY };
        let mut over = false;
        let counts = self.counts_all();
        // every (key, real (2, 2) entries, kind-0 entries) with its groups and root bound; visited best bound first
        struct Cfg { root: f64, code: u64, code_real22: usize, np: usize, filler: usize, plain: usize, gs: Vec<(usize, usize)>, gm: std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>>, rpk: std::rc::Rc<Vec<Vec<Vec<f64>>>> }
        let groups_all: Vec<&Vec<usize>> = by_kind.iter().chain(std::iter::once(pal_ents)).collect();
        // per group, its entries' `cv_mask`s ORed
        let gcvm: Vec<u64> = groups_all.iter().map(|l| l.iter().fold(0u64, |m, &j| m | self.cvm[j])).collect();
        // locked cards each group can hold (a configuration whose groups cannot hold them all is skipped)
        let (lbit, all_lk) = { let x = self.lock_bits(); (&x.0[..], x.1) };
        let glk: &[u8] = &lt.glk;
        let mut cfgs: Vec<Cfg> = vec![];
        let mut dw = vec![0.0f64; nu]; let mut dg = vec![0.0f64; nu];
        // (scratch of the configuration build: a candidate's groups, copied into its Cfg only when kept; the root's
        // clause sums)
        let mut gsb: Vec<(usize, usize)> = vec![]; let mut cvr = vec![0.0f64; clauses.len()];
        // (clause, chance, outfit) triples; the product value of one outfit at one chance is submodular in the set of
        // cards (log of each target's factor gains less from a card the more the deck already has), so a deck's value
        // is at most a partial deck's value times the best ratios P({card}) / P({}) of the cards still to add
        // (clause, trainee, chance, outfit), each (clause, trainee, chance) a run of its trainee's outfits: og
        let oq: Vec<(usize, usize, f64, usize)> = clauses.iter().enumerate().flat_map(|(ci, cl)| cl.0.iter().flat_map(move |&(k, q)| (0..model.n_outfits(us[k])).map(move |o| (ci, us[k], q, o)))).collect();
        let og: Vec<(usize, usize, usize)> = { let mut v = vec![]; let mut i = 0; while i < oq.len() { let no = model.n_outfits(oq[i].1); v.push((oq[i].0, i, no)); i += no; } v };
        let zero = vec![0.0f64; nu];
        // the node bound's per-target values, one per (clause, chance, outfit)
        let mut av = vec![0.0f64; oq.len()];
        // each (clause, chance, outfit)'s outfit, prepared once for the bounds below (`OutfitPre`: the same values as outfit_fast)
        let orow: Vec<crate::skills::OutfitPre> = oq.iter().map(|&(_, u, _, o)| model.outfit_pre(u, o)).collect();
        let p0: Vec<f64> = oq.iter().map(|&(_, u, q, o)| model.outfit_fast(u, o, q, &zero, &zero)).collect();
        // the root bound's ratio products (row 0 of the ratio suffix table, which is built only for configurations the
        // DFS expands)
        let mut sp0 = vec![1.0f64; oq.len()];
        // `make_rpk` below for the key with miss rows mw, mg (the same operations: the same values)
        let rpk_of = |mw: &[f64], mg: &[f64]| -> Vec<Vec<Vec<f64>>> {
            let (mut x, mut y, mut rat, mut r) = (vec![0.0f64; nu], vec![0.0f64; nu], Vec::<f64>::new(), Vec::<f64>::new());
            groups_all.iter().map(|l| {
                let noq = oq.len();
                rat.clear();
                for &j in l.iter() {
                    for t in 0..nu { x[t] = 1.0 - mw[j * nu + t]; y[t] = 1.0 - mg[j * nu + t]; }
                    rat.extend(oq.iter().enumerate().map(|(i, &(_, u, q, o))| if p0[i] > 0.0 { model.outfit_fast(u, o, q, &x, &y) / p0[i] } else { f64::INFINITY }));
                }
                (0..noq).map(|i| {
                    r.clear(); r.extend((0..l.len()).map(|e| rat[e * noq + i])); r.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    let mut pre = vec![1.0f64; size + 1];
                    for k in 1..=size { pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0); }
                    pre
                }).collect()
            }).collect() };
        // A configuration passing the per-target bound, its root not yet computed: `u` is an upper bound on its root
        // (the root sums fixed * max over outfits of min(A, B r) per (trainee, chance), the per-target value fixed *
        // the sum of max A, the same A to the bit; terms >= 0, so the root is at most the value up to rounding);
        // `key`: its key's slot in `pkeys` (the key's ratio tables, built on first need)
        struct Pend { u: f64, key: usize, code: u64, real22: usize, np: usize, filler: usize, plain: usize, gs: Vec<(usize, usize)>, gm: std::rc::Rc<Vec<Vec<(Vec<f64>, Vec<f64>)>>> }
        let mut pend: Vec<Pend> = vec![];
        // The key bound (`keyskip`; in the product search only with a bar): a key none of whose configurations (its
        // variants: real (2, 2) and pal entries 0 ..= their most) can pass the per-target bound is skipped whole, which
        // leaves out the same configurations. Its per-target products take, for the (2, 2) and pal groups, the lowest
        // miss products over the counts any variant has (row 0 is 1.0, a group a variant lacks), the others at the key's
        // counts, multiplied in the variants' order: with factors >= 0 each partial product is at most the variant's, so
        // its odds are at least every variant's, to the bit. The value rises with every odds coordinate (`valid`), and
        // each evaluation is within about 4e-14 per (trainee, chance) of exact (every quantity in it within [0, 1] up to
        // rounding, each operation's error at most 2^-53 of one, about 300 operations), so a variant's value is at most
        // the bound's plus `key_slack` (1e-11 per pair)
        let keyskip = valid && (!best || bestv > f64::NEG_INFINITY);
        let key_slack = |cl: &(Vec<(usize, f64)>, f64)| 1e-11 * cl.0.len() as f64 * fixed.max(1.0);
        let (mut kdw, mut kdg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        let mut pkeys: Vec<Option<std::rc::Rc<Vec<Vec<Vec<f64>>>>>> = vec![];
        for cnt in counts.iter() {
            let csum: usize = cnt.iter().sum();
            if (0..nk).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) { continue; }
            let mut pkey_new = true;
            let code: u64 = cnt.iter().zip(&kpos).map(|(&c, &p)| c as u64 * p).sum();
            let km = self.key_miss(code);
            let (mw, mg) = (&km.0, &km.1);
            // per kind group, the lowest miss chances of any entry under this key
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much)
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            // (the groups a configuration of this key can use: its kinds and, with slots left, the pal entries)
            let gneed: u64 = (0..nk).filter(|&i| cnt[i] > 0).fold(0u64, |m, i| m | 1 << i) | if csum < size { 1 << nk } else { 0 };
            let gm = self.gm_cached(set_id, code, &groups_all, gneed, mw, mg, nu, size);
            // the key's bound: no variant (real (2, 2) and pal entries counted 0 ..= their most) can pass when the value
            // at odds above every variant's fails with room for rounding (`key_slack`)
            if keyskip {
                let (r22, npm) = (cnt[ki22].min(by_kind[ki22].len()), (size - csum).min(pal_ents.len()));
                let lo = |i: usize, c: usize, t: usize| -> (f64, f64) { (0..=c).fold((f64::INFINITY, f64::INFINITY), |m, x| (m.0.min(gm[i][x].0[t]), m.1.min(gm[i][x].1[t]))) };
                for t in 0..nu {
                    let (mut a, mut b) = (1.0f64, 1.0f64);
                    if npm > 0 { let x = lo(nk, npm, t); a *= x.0; b *= x.1; }
                    for i in (0..nk).rev() {
                        let c = if i == ki22 { r22 } else { cnt[i] };
                        if c == 0 { continue; }
                        let x = if i == ki22 { lo(i, c, t) } else { (gm[i][c].0[t], gm[i][c].1[t]) };
                        a *= x.0; b *= x.1;
                    }
                    kdw[t] = 1.0 - a; kdg[t] = 1.0 - b;
                }
                // (a band's class caps are at most rho)
                let none = if best { (val(&clauses[0], &kdw, &kdg) + key_slack(&clauses[0])) * (1.0 + 1e-9) < bestv }
                    else { clauses.iter().all(|cl| rho * (val(cl, &kdw, &kdg) + key_slack(cl)) * (1.0 + 1e-9) < cl.1) };
                if none { continue; }
            }
            // per kind group and (clause, chance, outfit): products of the r best single-card ratios, r = 0..=size;
            // built only for keys with a configuration the per-target bound lets through
            let mut rpk: Option<std::rc::Rc<Vec<Vec<Vec<f64>>>>> = None;
            // (the entries' ratios in one flat scratch, rat[e * noq + i], and one scratch for each sorted column: the
            // same values, sorted the same way)
            let (mut x, mut y, mut rat, mut r) = (vec![0.0f64; nu], vec![0.0f64; nu], Vec::<f64>::new(), Vec::<f64>::new());
            let mut make_rpk = || -> Vec<Vec<Vec<f64>>> { groups_all.iter().map(|l| {
                let noq = oq.len();
                rat.clear();
                for &j in l.iter() {
                    for t in 0..nu { x[t] = 1.0 - mw[j * nu + t]; y[t] = 1.0 - mg[j * nu + t]; }
                    rat.extend(oq.iter().enumerate().map(|(i, &(_, u, q, o))| if p0[i] > 0.0 { model.outfit_fast(u, o, q, &x, &y) / p0[i] } else { f64::INFINITY }));
                }
                (0..noq).map(|i| {
                    r.clear(); r.extend((0..l.len()).map(|e| rat[e * noq + i])); r.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    let mut pre = vec![1.0f64; size + 1];
                    for k in 1..=size { pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0); }
                    pre
                }).collect()
            }).collect() };
            for real22 in 0..=cnt[ki22].min(by_kind[ki22].len()) {
                let filler = cnt[ki22] - real22;
                let rem = size - csum;
                for np in 0..=rem.min(pal_ents.len()) {
                    let plain = rem - np;
                    if plain > nplain { continue; }
                    if let Some(p) = self.dj.pals { if np + plain < p || plain > p { continue; } }
                    let gs = &mut gsb; gs.clear();
                    for i in 0..nk { let c = if i == ki22 { real22 } else { cnt[i] }; if c > 0 { gs.push((i, c)); } }
                    if np > 0 { gs.push((nk, np)); }
                    let gs = &gsb[..];
                    if all_lk & !gs.iter().fold(0u8, |m, &(i, _)| m | glk[i]) != 0 { continue; }
                    let ng = gs.len();
                    // the per-target bound first (row 0 of the suffix table the DFS builds, the same products in the same
                    // order), the ratio products only for configurations it lets through
                    for t in 0..nu {
                        let (mut a, mut b) = (1.0f64, 1.0f64);
                        for g in (0..ng).rev() { a *= gm[gs[g].0][gs[g].1].0[t]; b *= gm[gs[g].0][gs[g].1].1[t]; }
                        dw[t] = 1.0 - a; dg[t] = 1.0 - b;
                    }
                    // the configuration's class cap (every deck of it is in a class at most this one's)
                    let rc = if best { rho } else { ccap[cap_class(gs.iter().fold(0u64, |m, &(i, _)| m | gcvm[i]))] };
                    let iv = if best { val(&clauses[0], &dw, &dg) } else { 0.0 };
                    let ideal = if best { iv * (1.0 + 1e-9) >= bestv } else { clauses.iter().any(|cl| rc * val(cl, &dw, &dg) * (1.0 + 1e-9) >= cl.1) };
                    if !ideal { continue; }
                    if lazy {
                        if pkey_new { pkeys.push(None); pkey_new = false; }
                        pend.push(Pend { u: iv * (1.0 + 1e-12), key: pkeys.len() - 1, code, real22, np, filler, plain, gs: gs.to_vec(), gm: gm.clone() });
                        continue;
                    }
                    if rpk.is_none() { rpk = Some(std::rc::Rc::new(make_rpk())); }
                    let rpk = rpk.as_ref().unwrap();
                    // sp[0] of the suffix table: the same products in the same order (from the last group down)
                    for i in 0..oq.len() { let mut x = 1.0f64; for g in (0..ng).rev() { x *= rpk[gs[g].0][i][gs[g].1]; } sp0[i] = x; }
                    // root bound: per (chance, outfit) the lower of the per-target bound and the ratio bound
                    let cv = &mut cvr; cv.iter_mut().for_each(|x| *x = 0.0);
                    for &(ci, i, no) in &og {
                        let q = oq[i].2; let mut m = 0.0f64;
                        for o in 0..no { m = m.max(orow[i + o].eval(q, &dw, &dg).min(p0[i + o] * sp0[i + o])); }
                        cv[ci] += fixed * m;
                    }
                    let root = if best { cv[0] } else { (0..clauses.len()).map(|ci| rho * cv[ci] / clauses[ci].1.max(1e-300)).fold(f64::NEG_INFINITY, f64::max) };
                    // (the visiting order stays by `root`; a configuration whose root at its class cap fails is left out)
                    if !best && rc < rho && (0..clauses.len()).all(|ci| rc * cv[ci] * (1.0 + 1e-9) < clauses[ci].1) { continue; }
                    cfgs.push(Cfg { root, code, code_real22: real22, np, filler, plain, gs: gs.to_vec(), gm: gm.clone(), rpk: rpk.clone() });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        cfgs.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        SPT.with(|c| { let mut v = c.get(); v.0 += t_cfg.elapsed().as_secs_f64(); v.2 += cfgs.len(); v.3 += counts.len(); c.set(v); });
        let t_dfs = Instant::now();
        // A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
        // whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
        // is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
        // r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
        struct GB { own: Vec<(Vec<f64>, Vec<f64>)>, one: Vec<(Vec<f64>, Vec<f64>)>, rown: Vec<Vec<f64>>, rone: Vec<Vec<f64>> }
        let mut gbc: HashMap<(u64, usize), std::rc::Rc<GB>> = HashMap::new();
        let gb_of = |code: u64, gi: usize, mw: &[f64], mg: &[f64]| -> GB {
            let l = groups_all[gi];
            let (own, bor): (Vec<usize>, Vec<usize>) = l.iter().partition(|&&j| !self.ebor[j]);
            let mut om = vec![(vec![1.0f64; nu], vec![1.0f64; nu]); size + 1];
            let mut bm = (vec![1.0f64; nu], vec![1.0f64; nu]);
            for t in 0..nu {
                let mut a: Vec<f64> = own.iter().map(|&j| mw[j * nu + t]).collect(); a.sort_by(|x, y| x.partial_cmp(y).unwrap());
                let mut b: Vec<f64> = own.iter().map(|&j| mg[j * nu + t]).collect(); b.sort_by(|x, y| x.partial_cmp(y).unwrap());
                for r in 1..=size { om[r].0[t] = om[r - 1].0[t] * a.get(r - 1).copied().unwrap_or(1.0); om[r].1[t] = om[r - 1].1[t] * b.get(r - 1).copied().unwrap_or(1.0); }
                for &j in &bor { bm.0[t] = bm.0[t].min(mw[j * nu + t]); bm.1[t] = bm.1[t].min(mg[j * nu + t]); }
            }
            let mut one = om.clone();
            for r in 1..=size { for t in 0..nu { one[r].0[t] = om[r].0[t].min(om[r - 1].0[t] * bm.0[t]); one[r].1[t] = om[r].1[t].min(om[r - 1].1[t] * bm.1[t]); } }
            let (mut x, mut y) = (vec![0.0f64; nu], vec![0.0f64; nu]);
            let mut ratio = |j: usize| -> Vec<f64> {
                for t in 0..nu { x[t] = 1.0 - mw[j * nu + t]; y[t] = 1.0 - mg[j * nu + t]; }
                oq.iter().enumerate().map(|(i, &(_, u, q, o))| if p0[i] > 0.0 { model.outfit_fast(u, o, q, &x, &y) / p0[i] } else { f64::INFINITY }).collect()
            };
            let ro: Vec<Vec<f64>> = own.iter().map(|&j| ratio(j)).collect();
            let rb: Vec<Vec<f64>> = bor.iter().map(|&j| ratio(j)).collect();
            let (mut rown, mut rone) = (vec![], vec![]);
            for i in 0..oq.len() {
                let mut r: Vec<f64> = ro.iter().map(|v| v[i]).collect(); r.sort_by(|a, b| b.partial_cmp(a).unwrap());
                let best_b = rb.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
                let mut pre = vec![1.0f64; size + 1];
                for k in 1..=size { pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0); }
                let mut p1 = pre.clone();
                if best_b > f64::NEG_INFINITY { for k in 1..=size { p1[k] = pre[k].max(pre[k - 1] * best_b); } }
                rown.push(pre); rone.push(p1);
            }
            let _ = code;
            GB { own: om, one, rown, rone }
        };
        // the expanded configuration's suffix tables, flat and reused: per target (g * nu + t) the products of the
        // groups' lowest misses from group g on (white, gold), per (clause, chance, outfit) (g * noq + i) those of the
        // best ratios
        let noq = oq.len();
        let (mut sufw, mut sufg, mut spf): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        // the node bound's per-configuration constants, flat and reused (see `hopeful`)
        let (mut kab, mut kr, mut kab_have, mut kr_have): (Vec<f64>, Vec<f64>, Vec<bool>, Vec<bool>) = (vec![], vec![], vec![], vec![]);
        // the node bound's per-parent rows (see `hopeful`), per (depth, borrowing variant); parent ids never repeat
        // within a search
        struct Hc { pid: usize, fa: Vec<f64> }
        let mut hc: Vec<Hc> = vec![];
        let mut pid_next = 0usize;
        // (buffers of the expanded configuration, reused: clause sums, B's odds, the DFS rows, the locked-card table)
        let (mut cvx, mut pdwx, mut pdgx): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        let (mut st_sel, mut st_pw, mut st_pg): (Vec<usize>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        let mut avail_buf: Vec<Vec<u8>> = vec![];
        // (the class masks of the expanded configuration: csuf[goff[g] + p] = the `cv_mask`s ORed of group g's entries
        // from position p on and of every later group's)
        let (mut csuf, mut goff): (Vec<u64>, Vec<usize>) = (vec![], vec![]);
        // (a child's touched targets: their odds and the parent's factors they replace)
        let (mut tw, mut tg, mut told) = ([0.0f64; 32], [0.0f64; 32], [0.0f64; 32]);
        // the per-parent rows need every outfit row to read the nu targets
        let a_rows = orow.iter().all(|o| o.len() == nu);
        let oqq: Vec<f64> = oq.iter().map(|x| x.2).collect();
        // (lazy: the pending configurations by bound, highest first; the computed ones not yet visited, best root first
        // and, among equal roots, first built first, the order of the stable sort above)
        struct Hp { root: f64, idx: usize, cf: Cfg }
        impl PartialEq for Hp { fn eq(&self, o: &Self) -> bool { self.cmp(o) == std::cmp::Ordering::Equal } }
        impl Eq for Hp {}
        impl PartialOrd for Hp { fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> { Some(self.cmp(o)) } }
        impl Ord for Hp { fn cmp(&self, o: &Self) -> std::cmp::Ordering { self.root.partial_cmp(&o.root).unwrap().then(o.idx.cmp(&self.idx)) } }
        let mut pord: Vec<usize> = (0..pend.len()).collect();
        pord.sort_by(|&a, &b| pend[b].u.partial_cmp(&pend[a].u).unwrap());
        let mut heap: std::collections::BinaryHeap<Hp> = Default::default();
        let mut pp = 0usize;
        let (mut ldw, mut ldg, mut lsp0, mut lcv) = (vec![0.0f64; nu], vec![0.0f64; nu], vec![1.0f64; oq.len()], vec![0.0f64; 1]);
        let mut eager_i = 0usize;
        loop {
            let cf_own: Cfg;
            let cf: &Cfg = if lazy {
                // compute the pending configurations until the heap's top comes first in the sorted order: every one
                // left has a bound, so a root, below it (or cannot reach the bar)
                while pp < pord.len() {
                    let pe = &pend[pord[pp]];
                    if !(pe.u * (1.0 + 1e-9) >= bestv && heap.peek().map_or(true, |h| pe.u >= h.root)) { break; }
                    pp += 1;
                    // the same values as the eager build: the per-target bound, the key's ratio tables, sp0 and the root
                    let ng = pe.gs.len(); let (gs, gm) = (&pe.gs[..], &pe.gm);
                    for t in 0..nu {
                        let (mut a, mut b) = (1.0f64, 1.0f64);
                        for g in (0..ng).rev() { a *= gm[gs[g].0][gs[g].1].0[t]; b *= gm[gs[g].0][gs[g].1].1[t]; }
                        ldw[t] = 1.0 - a; ldg[t] = 1.0 - b;
                    }
                    if pkeys[pe.key].is_none() { let km = self.key_miss(pe.code); pkeys[pe.key] = Some(std::rc::Rc::new(rpk_of(&km.0, &km.1))); }
                    let rpk = pkeys[pe.key].as_ref().unwrap();
                    for i in 0..oq.len() { let mut x = 1.0f64; for g in (0..ng).rev() { x *= rpk[gs[g].0][i][gs[g].1]; } lsp0[i] = x; }
                    lcv[0] = 0.0;
                    for &(ci, i, no) in &og {
                        let q = oq[i].2; let mut m = 0.0f64;
                        for o in 0..no { m = m.max(orow[i + o].eval(q, &ldw, &ldg).min(p0[i + o] * lsp0[i + o])); }
                        lcv[ci] += fixed * m;
                    }
                    heap.push(Hp { root: lcv[0], idx: pord[pp - 1], cf: Cfg { root: lcv[0], code: pe.code, code_real22: pe.real22, np: pe.np, filler: pe.filler, plain: pe.plain, gs: pe.gs.clone(), gm: pe.gm.clone(), rpk: rpk.clone() } });
                }
                match heap.pop() { Some(h) => { cf_own = h.cf; &cf_own } None => break }
            } else { if eager_i >= cfgs.len() { break; } eager_i += 1; &cfgs[eager_i - 1] };
            if best && cf.root * (1.0 + 1e-9) < bestv { break; }
            if !best && cf.root * (1.0 + 1e-9) < 1.0 { break; }
            let km = self.key_miss(cf.code);
            let (mw, mg, tmk) = (&km.0, &km.1, &km.2[..]);
            let (filler, plain) = (cf.filler, cf.plain);
            let gs: Vec<(&[usize], usize)> = cf.gs.iter().map(|&(i, c)| (&groups_all[i][..], c)).collect();
            // per group of this configuration: its key's tables (gmin[g] = gm[group], rp[g] = rpk[group])
            let gmin: Vec<&Vec<(Vec<f64>, Vec<f64>)>> = cf.gs.iter().map(|&(i, _)| &cf.gm[i]).collect();
            let rp: Vec<&Vec<Vec<f64>>> = cf.gs.iter().map(|&(i, _)| &cf.rpk[i]).collect();
            // the same products in the same order as the per-configuration tables they replace
            {
                let ng = cf.gs.len();
                sufw.clear(); sufw.resize((ng + 1) * nu, 1.0); sufg.clear(); sufg.resize((ng + 1) * nu, 1.0);
                spf.clear(); spf.resize((ng + 1) * noq, 1.0);
                for g in (0..ng).rev() { let gmg = &cf.gm[cf.gs[g].0][cf.gs[g].1];
                    for t in 0..nu { sufw[g * nu + t] = sufw[(g + 1) * nu + t] * gmg.0[t]; sufg[g * nu + t] = sufg[(g + 1) * nu + t] * gmg.1[t]; } }
                for g in (0..ng).rev() { let rpg = &cf.rpk[cf.gs[g].0];
                    for i in 0..noq { spf[g * noq + i] = spf[(g + 1) * noq + i] * rpg[i][cf.gs[g].1]; } }
            }
            let (sufw, sufg, spf) = (&sufw[..], &sufg[..], &spf[..]);
            // the dominance tables of this configuration's groups (`dom_table`), and per flat index f (the groups' entries
            // in turn) the characters of the entries from f on
            let doms: Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>> = if dom_on { cf.gs.iter().map(|&(i, _)| self.dom_table(set_id, cf.code, i, groups_all[i], mw, mg, lbit)).collect() } else { vec![] };
            let (mut doff, mut dsuf): (Vec<usize>, Vec<u128>) = (vec![0], vec![]);
            if dom_on {
                for &(l, _) in &gs { for &j in l.iter() { dsuf.push(ebit[j]); } doff.push(dsuf.len()); }
                dsuf.push(0);
                for f in (0..dsuf.len() - 1).rev() { dsuf[f] |= dsuf[f + 1]; }
            }
            // the child taking entry B = list[p] of group g, at a node with `start`, `left` slots of g to fill, characters
            // `chm`, gold misses `ppg`, picks `sel` and characters `need` its decks must take: None when it can be
            // skipped, else its decks' needed characters. An entry A dominating B (`dom_table`) that is free at the node
            // and takes B's place in the pick order (a sibling, or no pick of g after A touching B's targets) gives every
            // deck through B whose picks after B take no card of A's character an equal or better deck before it; when
            // nothing of A's character can follow, that is every deck through B, else those decks must take one
            let dom = |g: usize, p: usize, start: usize, left: usize, chm: u128, ppg: &[f64], sel: &[usize], need: u128| -> Option<u128> {
                if !dom_on { return Some(need); }
                let l = gs[g].0; let bj = l[p]; let f = doff[g] + p;
                let mut nd = need;
                let dl = &doms[g][p];
                if !dl.is_empty() {
                    // (the deck's picks in group g so far, all at positions below `start`)
                    let sg = &sel[sel.len() - (gs[g].1 - left)..];
                    for &(a, sm) in dl.iter() {
                        let a = a as usize;
                        let aj = l[a];
                        if chm & ebit[aj] != 0 { continue; }
                        if a < start && sg.iter().any(|&j| j > aj && tmk[j] & tmk[bj] != 0) { continue; }
                        if sm != 0 {
                            let mut ok = true; let mut m = sm;
                            while m != 0 { let t = m.trailing_zeros() as usize; m &= m - 1;
                                let rlo = gmin[g][left - 1].1[t] * sufg[(g + 1) * nu + t];
                                if !(ppg[t] * rlo * (mg[bj * nu + t] - mg[aj * nu + t]) >= DOM_GAP) { ok = false; break; } }
                            if !ok { continue; }
                        }
                        if ebit[aj] != ebit[bj] && ebit[aj] & dsuf[f + 1] != 0 { nd |= ebit[aj]; continue; }
                        return None;
                    }
                }
                // a needed character that no later pick can bring
                if nd & !(chm | ebit[bj]) & !dsuf[f + 1] != 0 { return None; }
                Some(nd)
            };
            // the one-borrowed-card tables of this configuration's groups and their suffixes (`own`: no borrowed card
            // left to place; `one`: at most one, in one group)
            let one_b = self.allowed_borrow == 1 && gs.iter().any(|(l, _)| l.iter().any(|&j| self.ebor[j]));
            let gbs: Vec<std::rc::Rc<GB>> = if one_b { cf.gs.iter().map(|&(i, _)| gbc.entry((cf.code, i)).or_insert_with(|| std::rc::Rc::new(gb_of(cf.code, i, mw, mg))).clone()).collect() } else { vec![] };
            let ngs = cf.gs.len();
            // (allocated only when used)
            let nb = if one_b { ngs + 1 } else { 0 };
            // (vec![x; 0] would still build x: none without borrowing)
            let (mut so, mut s1) = if nb > 0 { (vec![(vec![1.0f64; nu], vec![1.0f64; nu]); nb], vec![(vec![1.0f64; nu], vec![1.0f64; nu]); nb]) } else { (vec![], vec![]) };
            let (mut po, mut p1) = if nb > 0 { (vec![vec![1.0f64; oq.len()]; nb], vec![vec![1.0f64; oq.len()]; nb]) } else { (vec![], vec![]) };
            if one_b { for g in (0..ngs).rev() { let (gb, c) = (&gbs[g], cf.gs[g].1);
                for t in 0..nu {
                    so[g].0[t] = gb.own[c].0[t] * so[g + 1].0[t]; so[g].1[t] = gb.own[c].1[t] * so[g + 1].1[t];
                    s1[g].0[t] = (gb.one[c].0[t] * so[g + 1].0[t]).min(gb.own[c].0[t] * s1[g + 1].0[t]);
                    s1[g].1[t] = (gb.one[c].1[t] * so[g + 1].1[t]).min(gb.own[c].1[t] * s1[g + 1].1[t]);
                }
                for i in 0..oq.len() { po[g][i] = gb.rown[i][c] * po[g + 1][i]; p1[g][i] = (gb.rone[i][c] * po[g + 1][i]).max(gb.rown[i][c] * p1[g + 1][i]); }
            } }
            cvx.clear(); cvx.resize(clauses.len(), 0.0); pdwx.clear(); pdwx.resize(nu, 0.0); pdgx.clear(); pdgx.resize(nu, 0.0);
            let (cv, pdw, pdg) = (&mut cvx[..], &mut pdwx[..], &mut pdgx[..]);
            let ng = gs.len();
            if !best {
                goff.clear(); goff.push(0); for g in 0..ng { goff.push(goff[g] + gs[g].0.len()); }
                csuf.clear(); csuf.resize(goff[ng] + 1, 0);
                for g in (0..ng).rev() { for (p, &j) in gs[g].0.iter().enumerate().rev() { csuf[goff[g] + p] = csuf[goff[g] + p + 1] | self.cvm[j]; } }
            }
            let (csuf, goff) = (&csuf[..], &goff[..]);
            // the state-conditioned bound of this configuration: chain positions (sorted kinds, the key's state
            // distribution projected on the positions whose group has chain targets), per group its slots' positions
            // and, per chain level, each target's lowest misses over the group's entries (a slot may take any of them,
            // the same one twice: a relaxation)
            let sbkey = (set_id, cf.code, cf.code_real22, cf.np);
            let sbc: Option<std::rc::Rc<SbCfg>> = if sbm == 0 { None } else if let Some(r) = self.sbcfg.borrow().get(&sbkey).cloned() { Some(r) } else {
                let mut kinds: Vec<(Kind, usize)> = vec![];   // (kind, group, or usize::MAX for a filler)
                for (g, &(i, c)) in cf.gs.iter().enumerate() { if i < nk { for _ in 0..c { kinds.push((self.kinds_all[i], g)); } } }
                for _ in 0..filler { kinds.push(((2, 2, 0), usize::MAX)); }
                kinds.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                let kl: Vec<Kind> = kinds.iter().map(|x| x.0).collect();
                let full = if kl.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kl) };
                let has_chain: Vec<bool> = gs.iter().map(|(l, _)| l.iter().any(|&j| !self.srcs[j].cm.is_empty())).collect();
                let rel_pos: Vec<usize> = (0..kinds.len()).filter(|&p| kinds[p].1 != usize::MAX && has_chain[kinds[p].1]).collect();
                let mut slots: Vec<Vec<usize>> = vec![vec![]; ng];
                for (ri, &p) in rel_pos.iter().enumerate() { slots[kinds[p].1].push(ri); }
                for g in 0..ng { while slots[g].len() < gs[g].1 { slots[g].push(usize::MAX); } }
                let mut proj: crate::FxMap<Vec<u8>, f64> = Default::default();
                for (x, w) in full.iter() { let key: Vec<u8> = rel_pos.iter().map(|&p| x[p]).collect(); *proj.entry(key).or_insert(0.0) += w; }
                let mut states: Vec<(Vec<u8>, f64)> = proj.into_iter().collect(); states.sort_by(|a, b| a.0.cmp(&b.0));
                let stops: Vec<usize> = cf.gs.iter().map(|&(i, _)| if i < nk { self.kinds_all[i].1 as usize } else { 0 }).collect();
                let gmin: Vec<Vec<(f64, f64)>> = (0..ng).map(|g| {
                    let mut m = vec![(1.0f64, 1.0f64); (stops[g] + 1) * nu];
                    for &j in gs[g].0 { if let Some(r) = &sb_rows[j] { for (x, y) in m.iter_mut().zip(r.iter()) { x.0 = x.0.min(y.0); x.1 = x.1.min(y.1); } } }
                    m }).collect();
                let r = std::rc::Rc::new(SbCfg { states, slots, gmin });
                self.sbcfg.borrow_mut().insert(sbkey, r.clone());
                Some(r)
            };
            // the bound at a partial deck `sel` (entries in pick order, filling the groups in turn)
            let sb_ub = |sel: &[usize]| -> f64 {
                let c = sbc.as_ref().unwrap();
                let (mut kg, mut kh) = ([1.0f64; 32], [1.0f64; 32]);
                let mut chain: Vec<(usize, &[(f64, f64)])> = vec![];
                let mut idx = 0usize;
                for g in 0..ng { for t in 0..gs[g].1 {
                    let rows: &[(f64, f64)] = if idx < sel.len() { sb_rows[sel[idx]].as_deref().unwrap() } else { &c.gmin[g] };
                    idx += 1;
                    let p = c.slots[g][t];
                    if p == usize::MAX { for k in 0..nu { kg[k] *= rows[k].0; kh[k] *= rows[k].1; } } else { chain.push((p, rows)); }
                } }
                let opts = &model.opts[sb_u];
                let mut acc = [0.0f64; 16];
                let (mut xg, mut xh) = ([0.0f64; 32], [0.0f64; 32]);
                for (x, w) in &c.states {
                    xg[..nu].copy_from_slice(&kg[..nu]); xh[..nu].copy_from_slice(&kh[..nu]);
                    for &(p, rows) in &chain { let b = x[p] as usize * nu; for k in 0..nu { xg[k] *= rows[b + k].0; xh[k] *= rows[b + k].1; } }
                    for (o, (st, _)) in opts.iter().enumerate() {
                        let ew = &model.ews[sb_u][o];
                        let mut f = 1.0f64; for k in 0..nu { f *= land_factor(st[k], ew[k], sb_q, xg[k], xh[k]); }
                        acc[o] += w * f;
                    }
                }
                fixed * acc[..opts.len()].iter().cloned().fold(0.0f64, f64::max)
            };
            // the configuration's root: no deck of it can reach the bar
            let sb_root = if sbm > 0 { sb_ub(&[]) } else { 0.0 };
            if sbm > 0 && sb_root * (1.0 + 1e-9) < sb_bar { SBN.with(|c| c.borrow_mut().0 += 1); continue; }
            {
                {
                    // the partial decks' miss products, one row of `nu` per depth (the last row: the deck so far)
                    struct St { sel: Vec<usize>, pw: Vec<f64>, pg: Vec<f64>, pid: usize }
                    // (pw, pg: one row of nu per depth 0 ..= the deck's slots, row d at d * nu: the partial deck with d cards)
                    let nrows = gs.iter().map(|x| x.1).sum::<usize>() + 1;
                    let mut st = St { sel: std::mem::take(&mut st_sel), pw: std::mem::take(&mut st_pw), pg: std::mem::take(&mut st_pg), pid: pid_next };
                    st.sel.clear(); st.pw.clear(); st.pw.resize(nrows * nu, 1.0); st.pg.clear(); st.pg.resize(nrows * nu, 1.0);
                    let nhc = 2 * (gs.iter().map(|x| x.1).sum::<usize>() + 1);
                    while hc.len() < nhc { hc.push(Hc { pid: usize::MAX, fa: vec![0.0; noq * nu] }); }
                    // per (group g, cards left, borrowing allowed): the per-target suffix factors (a, b) and the per-outfit
                    // ratio products r depend on nothing else, so they are constants of the configuration: tabulated on
                    // first use of their row (the same expressions, so the same f64s as computing them per node); row
                    // (loff[g] + left) * ncan + can, a row of 2 nu factors (a then b) and one of noq ratios
                    let ncan = if one_b { 2 } else { 1 };
                    let mut loff = vec![0usize; ng + 1]; for g in 0..ng { loff[g + 1] = loff[g] + gs[g].1 + 1; }
                    let nrow = loff[ng] * ncan;
                    kab.clear(); kab.resize(nrow * 2 * nu, 0.0); kr.clear(); kr.resize(nrow * noq, 0.0);
                    kab_have.clear(); kab_have.resize(nrow, false); kr_have.clear(); kr_have.resize(nrow, false);
                    // can a deck from here (group g with `left` still to pick, `pw`/`pg` the partial miss) pass?
                    // (ppw, ppg: the parent's row; dep, pid: the parent's depth and id; tm: the targets the child's card
                    // touches; cm, p: the child's class mask (`cv_mask`s ORed) and its card's position in group g)
                    let mut hopeful = |g: usize, left: usize, pw: &[f64], pg: &[f64], bestv: f64, nb: usize, ppw: &[f64], ppg: &[f64], dep: usize, pid: usize, tm: u32, cm: u64, p: usize| -> bool {
                        // the cap of every deck the child can still reach: its cards' classes and every entry left
                        let rho = if best { rho } else { ccap[cap_class(cm | if left > 0 { csuf[goff[g] + p + 1] } else { csuf[goff[g + 1]] })] };
                        // borrowed cards still allowed (only with the one-borrowed tables)
                        let can = nb < 1;
                        let mut have_a = false;
                        let row = if g >= ng { usize::MAX } else { (loff[g] + left) * ncan + (one_b && can) as usize };
                        let (pw, pg) = (&pw[..nu], &pg[..nu]);
                        if row == usize::MAX {
                            for t in 0..nu { dw[t] = 1.0 - pw[t] * 1.0; dg[t] = 1.0 - pg[t] * 1.0; pdw[t] = 1.0 - pw[t]; pdg[t] = 1.0 - pg[t]; }
                        } else {
                            if !kab_have[row] {
                                kab_have[row] = true;
                                for t in 0..nu {
                                    let (a, b) = if !one_b { (gmin[g][left].0[t] * sufw[(g + 1) * nu + t], gmin[g][left].1[t] * sufg[(g + 1) * nu + t]) }
                                        else if can { let gb = &gbs[g];
                                            ((gb.one[left].0[t] * so[g + 1].0[t]).min(gb.own[left].0[t] * s1[g + 1].0[t]), (gb.one[left].1[t] * so[g + 1].1[t]).min(gb.own[left].1[t] * s1[g + 1].1[t])) }
                                        else { (gbs[g].own[left].0[t] * so[g + 1].0[t], gbs[g].own[left].1[t] * so[g + 1].1[t]) };
                                    kab[row * 2 * nu + t] = a; kab[row * 2 * nu + nu + t] = b;
                                }
                            }
                            let (ka, kb) = kab[row * 2 * nu..(row + 1) * 2 * nu].split_at(nu);
                            if a_rows {
                                // A child is its parent plus one card: for a target the card does not touch (its miss
                                // chances exactly 1.0) the child's pw[t] = parent's pw[t] * 1.0 is the parent's, bit for
                                // bit, and so are dw[t], dg[t] and every outfit's A factor of t (the same expressions on
                                // the same f64s; ka, kb depend only on the row, the same for all children of one parent
                                // and borrowing variant). So the factors are computed once per parent (here, at its first
                                // child), a child recomputes only its touched targets', and A is the product in target
                                // order (`OutfitPre::eval`'s)
                                let c = &mut hc[dep * 2 + (one_b && can) as usize];
                                if c.pid != pid {
                                    c.pid = pid;
                                    let (mut bw, mut bg) = ([0.0f64; 32], [0.0f64; 32]);
                                    for t in 0..nu { bw[t] = 1.0 - ppw[t] * ka[t]; bg[t] = 1.0 - ppg[t] * kb[t]; }
                                    for ix in 0..noq { orow[ix].factors(oqq[ix], &bw[..nu], &bg[..nu], &mut c.fa[ix * nu..(ix + 1) * nu]); }
                                }
                                let mut m = tm;
                                while m != 0 { let t = m.trailing_zeros() as usize; m &= m - 1; tw[t] = 1.0 - pw[t] * ka[t]; tg[t] = 1.0 - pg[t] * kb[t]; }
                                for (ix, row) in c.fa.chunks_exact_mut(nu).enumerate() {
                                    let (o, q) = (&orow[ix], oqq[ix]);
                                    let mut m = tm;
                                    while m != 0 { let t = m.trailing_zeros() as usize; m &= m - 1; told[t] = row[t]; row[t] = o.factor_at(t, q, tw[t], tg[t]); }
                                    let mut p = 1.0f64; for &x in row.iter() { p *= x; }
                                    let mut m = tm;
                                    while m != 0 { let t = m.trailing_zeros() as usize; m &= m - 1; row[t] = told[t]; }
                                    av[ix] = p;
                                }
                                // (B's inputs pdw, pdg: below, only when B is evaluated)
                                have_a = true;
                            } else {
                                let (dw, dg, pdw, pdg) = (&mut dw[..nu], &mut dg[..nu], &mut pdw[..nu], &mut pdg[..nu]);
                                for t in 0..nu { dw[t] = 1.0 - pw[t] * ka[t]; dg[t] = 1.0 - pg[t] * kb[t]; pdw[t] = 1.0 - pw[t]; pdg[t] = 1.0 - pg[t]; }
                            }
                        }
                        let pass = |cv: &[f64]| if best { cv[0] * (1.0 + 1e-9) >= bestv }
                            else { (0..clauses.len()).any(|ci| rho * cv[ci] * (1.0 + 1e-9) >= clauses[ci].1) };
                        // the per-target values A first (each outfit's term is min(A, B r) <= A, so the clause sums with
                        // A alone are upper bounds of the real ones: when they fail, so does the real test; A is finite,
                        // its inputs being probabilities, and a NaN falls through to the real test)
                        cv.iter_mut().for_each(|x| *x = 0.0);
                        let mut nan = false;
                        for &(ci, i, no) in &og {
                            let q = oq[i].2; let mut m = 0.0f64;
                            for o in 0..no { let a = if have_a { av[i + o] } else { orow[i + o].eval(q, &dw, &dg) }; av[i + o] = a; nan |= a.is_nan(); m = m.max(a); }
                            cv[ci] += fixed * m;
                        }
                        if !nan && !pass(&cv) { return false; }
                        if row != usize::MAX && !kr_have[row] {
                            kr_have[row] = true;
                            for ix in 0..noq {
                                kr[row * noq + ix] = if !one_b { rp[g][ix][left] * spf[(g + 1) * noq + ix] }
                                    else if can { (gbs[g].rone[ix][left] * po[g + 1][ix]).max(gbs[g].rown[ix][left] * p1[g + 1][ix]) }
                                    else { gbs[g].rown[ix][left] * po[g + 1][ix] };
                            }
                        }
                        if have_a { for t in 0..nu { pdw[t] = 1.0 - pw[t]; pdg[t] = 1.0 - pg[t]; } }
                        // the real bound, the same expression in the same order; an outfit whose A is at most the running
                        // maximum leaves it unchanged (min(A, B r) <= A <= m) and is skipped
                        cv.iter_mut().for_each(|x| *x = 0.0);
                        for &(ci, i, no) in &og {
                            let q = oq[i].2; let mut m = 0.0f64;
                            for o in 0..no {
                                let ix = i + o;
                                let a = av[ix];
                                if a <= m { continue; }
                                let r = if row == usize::MAX { 1.0 } else { kr[row * noq + ix] };
                                m = m.max(a.min(orow[ix].eval(q, &pdw, &pdg) * r));
                            }
                            cv[ci] += fixed * m;
                        }
                        pass(&cv)
                    };
                    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a
                    // later group (a branch that can no longer place every locked card is dropped)
                    let all_locked = all_lk;
                    let mut avail = std::mem::take(&mut avail_buf);
                    avail.truncate(ng + 1); avail.resize_with(ng + 1, Vec::new);
                    avail[ng].clear(); avail[ng].push(0);
                    for g in (0..ng).rev() {
                        let l = gs[g].0; let next = avail[g + 1][0];
                        let v = &mut avail[g]; v.clear(); v.resize(l.len() + 1, 0u8); v[l.len()] = next;
                        for p in (0..l.len()).rev() { v[p] = v[p + 1] | lbit[l[p]]; }
                    }
                    if all_locked & !avail[0][0] != 0 { avail_buf = avail; st_sel = st.sel; st_pw = st.pw; st_pg = st.pg; continue; }
                    fn rec(sp: &DeckSpace, gs: &[(&[usize], usize)], g: usize, start: usize, left: usize, chm: u128, nb: usize, have: u8, cmk: u64, lk: (&[u8], u8, &[Vec<u8>]), st: &mut St, ebit: &[u128],
                           mw: &[f64], mg: &[f64], tmk: &[u32], nu: usize, hopeful: &mut dyn FnMut(usize, usize, &[f64], &[f64], f64, usize, &[f64], &[f64], usize, usize, u32, u64, usize) -> bool,
                           leaf: &mut dyn FnMut(&[usize], &[f64], &[f64], u64) -> bool, bestv: &dyn Fn() -> f64, sbx: &mut dyn FnMut(&[usize]) -> bool,
                           dom: &dyn Fn(usize, usize, usize, usize, u128, &[f64], &[usize], u128) -> Option<u128>, need: u128) -> bool {
                        let (lbit, all_locked, avail) = lk;
                        if left == 0 {
                            if g + 1 >= gs.len() { if need & !chm != 0 { return false; } let b = st.sel.len() * nu; return leaf(&st.sel, &st.pw[b..b + nu], &st.pg[b..b + nu], cmk); }
                            return rec(sp, gs, g + 1, 0, gs[g + 1].1, chm, nb, have, cmk, lk, st, ebit, mw, mg, tmk, nu, hopeful, leaf, bestv, sbx, dom, need);
                        }
                        if all_locked & !have & !avail[g][start] != 0 { return false; }
                        let list = gs[g].0;
                        // the deck so far is row sel.len(); a child's row is the next one (overwritten per child)
                        let b = st.sel.len() * nu;
                        // this parent's id and depth (the per-parent rows of `hopeful`)
                        st.pid += 1; let (pid, dep) = (st.pid, st.sel.len());
                        for p in start..list.len() {
                            if list.len() - p < left { break; }
                            let j = list[p];
                            if chm & ebit[j] != 0 || (sp.ebor[j] && nb >= sp.allowed_borrow) { continue; }
                            let Some(nd) = dom(g, p, start, left, chm, &st.pg[b..b + nu], &st.sel, need) else { continue };
                            sp.nodes.set(sp.nodes.get() + 1);
                            // the child's row (depth + 1): the parent's times the card's misses
                            { let (par, ch) = st.pw.split_at_mut(b + nu); let (mj, par, ch) = (&mw[j * nu..(j + 1) * nu], &par[b..], &mut ch[..nu]);
                              for t in 0..nu { ch[t] = par[t] * mj[t]; } }
                            { let (par, ch) = st.pg.split_at_mut(b + nu); let (mj, par, ch) = (&mg[j * nu..(j + 1) * nu], &par[b..], &mut ch[..nu]);
                              for t in 0..nu { ch[t] = par[t] * mj[t]; } }
                            let cj = cmk | sp.cvm[j];
                            if !hopeful(g, left - 1, &st.pw[b + nu..b + 2 * nu], &st.pg[b + nu..b + 2 * nu], bestv(), nb + sp.ebor[j] as usize, &st.pw[b..b + nu], &st.pg[b..b + nu], dep, pid, tmk[j], cj, p) { continue; }
                            st.sel.push(j);
                            if !sbx(&st.sel) { st.sel.pop(); continue; }
                            let stop = rec(sp, gs, g, p + 1, left - 1, chm | ebit[j], nb + sp.ebor[j] as usize, have | lbit[j], cj, lk, st, ebit, mw, mg, tmk, nu, hopeful, leaf, bestv, sbx, dom, nd);
                            st.sel.pop();
                            if stop { return true; }
                        }
                        false
                    }
                    // the state-conditioned bound at depth <= 1 (mode 1) or at every depth, the leaf included (mode 2); with
                    // SB_CHECK, every leaf's value against its root's and depth-1 node's bounds
                    let total_slots: usize = gs.iter().map(|x| x.1).sum();
                    let mut sb_d1 = f64::INFINITY;
                    let mut sbx = |sel: &[usize]| -> bool {
                        if sbm == 0 || (sbm == 3 && !sb_check()) { return true; }
                        let d = sel.len();
                        let chk = sb_check();
                        let leafd = d == total_slots;
                        if (sbm == 1 || sbm == 3) && d > 1 && !(chk && leafd) { return true; }
                        if sbm == 3 && d == 1 && !chk { return true; }
                        let ub = sb_ub(sel);
                        SBN.with(|c| c.borrow_mut().1 += 1);
                        if d == 1 { sb_d1 = ub; }
                        if chk && leafd { SBCHK.with(|c| { let mut c = c.borrow_mut(); c.4 += 1; if ub > sb_root * (1.0 + 1e-9) || (d > 1 && ub > sb_d1 * (1.0 + 1e-9)) { c.5 += 1; } }); }
                        if (sbm == 1 && d > 1) || sbm == 3 { return true; }
                        let pass = ub * (1.0 + 1e-9) >= sb_bar;
                        if !pass { SBN.with(|c| c.borrow_mut().2 += 1); }
                        pass
                    };
                    let bcell = std::cell::Cell::new(bestv);
                    // the deck's odds, written per leaf (no allocation unless the deck is kept)
                    let (mut dwa, mut dga) = ([0.0f64; 32], [0.0f64; 32]);
                    let mut leaf = |sel: &[usize], pw: &[f64], pg: &[f64], cmk: u64| -> bool {
                        let pals = sel.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + plain;
                        if let Some(p) = self.dj.pals { if p != pals { return false; } }
                        if !self.locked.iter().all(|l| sel.iter().any(|&j| self.rel[j] == *l)) { return false; }
                        for t in 0..nu { dwa[t] = 1.0 - pw[t]; dga[t] = 1.0 - pg[t]; }
                        let (dw, dg) = (&dwa[..nu], &dga[..nu]);
                        let mk = |v: f64| { let mut e = sel.to_vec(); e.sort(); (v, Sel { ents: e, plain, filler, dw: dw.to_vec(), dg: dg.to_vec() }) };
                        if best {
                            let v = val(&clauses[0], dw, dg);
                            if v > bcell.get() { bcell.set(v); out.clear(); out.push(mk(v)); }
                            false
                        } else {
                            // each clause's value once: the best of them, and whether any passes its bar at the deck's class cap
                            let (mut v, mut pass) = (f64::NEG_INFINITY, false);
                            let rho = ccap[cap_class(cmk)];
                            for cl in clauses { let x = val(cl, dw, dg); v = v.max(x); pass = pass || rho * x * (1.0 + 1e-9) >= cl.1; }
                            if pass {
                                out.push(mk(v));
                                if out.len() >= cap { return true; }
                            }
                            false
                        }
                    };
                    let bget = || bcell.get();
                    let stop = if gs.is_empty() { leaf(&[], &[1.0; 32][..nu], &[1.0; 32][..nu], 0) }
                        else { rec(self, &gs, 0, 0, gs[0].1, 0, 0, 0, 0, (lbit, all_locked, &avail), &mut st, ebit, mw, mg, tmk, nu, &mut hopeful, &mut leaf, &bget, &mut sbx, &dom, 0) };
                    pid_next = st.pid;
                    avail_buf = avail; st_sel = st.sel; st_pw = st.pw; st_pg = st.pg;
                    bestv = bcell.get();
                    if stop { over = true; return (out, over); }
                }
            }
        }
        SPT.with(|c| { let mut v = c.get(); v.1 += t_dfs.elapsed().as_secs_f64(); c.set(v); });
        (out, over)
    }
    /// The product search's dominance table of group `gi` (entries `l`) under key `code`: per position p, the earlier
    /// positions a whose entry A could take B = l[p]'s place in any deck (borrowed only if B is, B's locked card if B
    /// is one, B's pal status when the job fixes the pals) with every miss chance in [0, 1] and at most B's, white and
    /// gold, for every target; with the targets where A's gold miss is the lower (bit t).
    ///
    /// Why the product search (`best`) may skip a child B of a node (`dom` in `search`): let A be such an entry, free at
    /// the node (no pick of its character) and taking B's place in the pick order: a sibling (position in [start, p),
    /// right after the node's picks of group g) or an earlier one no later pick of g touches B's targets with (their
    /// factors are exactly 1.0 there). Any deck D through B and D' = D - B + A then multiply their miss chances in the
    /// same order but for that one factor, so with rounding monotone every miss product of D' is at most D's: every
    /// white odd at least D's, every gold odd equal to D's (equal gold misses) or above it by more than any rounding
    /// (`DOM_GAP`). The value is monotone in the white odds operation by operation, and a gold odd's factor rises by at
    /// least (GEN_GOLD - GEN_WHITE) times its gap, far above its rounding; so val(D') >= val(D) as computed. D' is legal
    /// when no pick of D after B is of A's character (nor of B's: A shares it or is free) and comes before D in the
    /// enumeration (the same configuration, lexicographically lower). The search returns the first deck of the highest
    /// value in enumeration order (strictly higher replaces it; a branch is pruned only below the best so far), which is
    /// never such a D: the decks through B are skipped when nothing of A's character can follow B, else those without a
    /// pick of A's character (`need`). Only which decks are visited (the rest in the same order) and how the bar rises
    /// change: the deck found, its value and odds do not.
    pub(super) fn dom_table(&self, set_id: usize, code: u64, gi: usize, l: &[usize], mw: &[f64], mg: &[f64], lbit: &[u8]) -> std::rc::Rc<Vec<Vec<(u32, u32)>>> {
        if let Some(r) = self.domc.borrow().get(&(set_id, code, gi)) { return r.clone(); }
        let nu = self.unc.len();
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let ok = |j: usize| (0..nu).all(|t| in01(mw[j * nu + t]) && in01(mg[j * nu + t]));
        let pal = |j: usize| self.cards[self.rel[j]].pal;
        let r = std::rc::Rc::new((0..l.len()).map(|p| {
            let b = l[p];
            if !ok(b) { return vec![]; }
            (0..p).filter_map(|a| {
                let aj = l[a];
                if self.ebor[aj] && !self.ebor[b] { return None; }
                if lbit[b] != 0 && lbit[aj] != lbit[b] { return None; }
                if self.dj.pals.is_some() && pal(aj) != pal(b) { return None; }
                if !ok(aj) || !(0..nu).all(|t| mw[aj * nu + t] <= mw[b * nu + t] && mg[aj * nu + t] <= mg[b * nu + t]) { return None; }
                Some((a as u32, (0..nu).filter(|&t| mg[aj * nu + t] < mg[b * nu + t]).fold(0u32, |m, t| m | 1 << t)))
            }).collect()
        }).collect::<Vec<_>>());
        self.domc.borrow_mut().insert((set_id, code, gi), r.clone());
        r
    }
}
/// The least exact gap (parent's gold-miss product times the picks left's lowest times the misses' difference) for
/// which a lower gold miss is sure to give a gold odd and factor above the other deck's after rounding (both are
/// within about 2^-50 of exact)
const DOM_GAP: f64 = 1.0 / (1u64 << 28) as f64;
