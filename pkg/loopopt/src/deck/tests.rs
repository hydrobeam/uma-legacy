use super::*;

fn fixture_skills() -> SkillsRaw {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/web_skills.json"))
        .expect("read global skill fixture");
    serde_json::from_str(&text).expect("parse global skill fixture")
}

fn fixture_cards() -> CardsRaw {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/web_cards.json"))
        .expect("read global card fixture");
    serde_json::from_str(&text).expect("parse global card fixture")
}

/// The deck search's allocation-free lineage chances and per-trainee odds equal the reference forms.
#[test]
fn fast_metric_matches_reference() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":21010},{"group":20165}]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let nu = m.uncertain.len();
    let fs: [&[usize]; 5] = [&[0, 5, 9, 20], &[3, 7, 11, 40], &[1, 2, 3, 4], &[0, 5, 9], &[3, 11, 40]];
    for (fi, &f) in fs.iter().enumerate() {
        let qa = m.lineage_q_arr(f, &|_, _| 3.0);
        let qv = m.lineage_q(f, &|_, _| 3.0);
        assert_eq!(qv.len(), if f.len() == 4 { 12 } else { 3 });
        assert!(qa.iter().zip(&qv).all(|(a, b)| a.to_bits() == b.to_bits()));
        let dw: Vec<f64> = (0..nu).map(|k| ((k + fi) as f64 * 0.17) % 1.0).collect();
        let dg: Vec<f64> = (0..nu).map(|k| ((k * 3 + fi) as f64 * 0.11) % 1.0).collect();
        // the search's per-trainee term (best outfit, no allocation) against the detail view's (`trainee_d`): one
        // landing form (`land_dw`) multiplied in target order, so the same f64
        for (&(t, ..), &q) in skills::lins(f.len()).iter().zip(&qv) {
            let (a, b) = (m.trainee_fast(f[t], q, &dw, &dg), m.trainee_d(f[t], q, &dw, &dg).0);
            assert_eq!(a.to_bits(), b.to_bits(), "{a} vs {b}");
        }
    }
}
/// dense chain_marginals equals the reference to float noise
#[test]
fn chain_marginals_dense_matches_ref() {
    let kinds: Vec<Kind> =
        vec![(2, 2, 0), (3, 3, 0), (3, 2, 0), (3, 1, 0), (4, 4, 0), (4, 3, 0), (5, 5, 0), (5, 4, 0), (5, 2, 0)];
    let mut worst = 0.0f64;
    let mut cnt = 0;
    fn visit_multisets(
        kinds: &[Kind],
        start: usize,
        remaining: usize,
        current: &mut Vec<Kind>,
        visit: &mut dyn FnMut(&[Kind]),
    ) {
        if !current.is_empty() {
            visit(current);
        }
        if remaining == 0 {
            return;
        }
        for i in start..kinds.len() {
            current.push(kinds[i]);
            visit_multisets(kinds, i, remaining - 1, current, visit);
            current.pop();
        }
    }
    visit_multisets(&kinds, 0, 6, &mut vec![], &mut |ks| {
        let a = chain_marginals_ref(ks);
        let b = chain_marginals_dense(ks);
        cnt += 1;
        for (k, v) in &a {
            for (x, y) in v.iter().zip(&b[k]) {
                worst = worst.max((x - y).abs());
            }
        }
    });
    eprintln!("{cnt} multisets, max |dense - ref| = {worst:e}");
    assert!(worst < 1e-14);
}
/// The character rule: a card blocks its own character and, for a group card, every member.
#[test]
fn card_blocks_own_character_and_group_members() {
    let cr: CardsRaw = fixture_cards();
    let sirius = cr.cards.iter().find(|c| c.id == 30081).expect("Team Sirius");
    assert!(sirius.blocks(1001) && sirius.blocks(1002) && !sirius.blocks(1008)); // Special Week, Suzuka; not Vodka
    let halo = cr.cards.iter().find(|c| c.name.contains("Tonight, We Waltz")).unwrap();
    assert!(halo.blocks(halo.chara) && !halo.blocks(1001));
}
/// Calibration against the uma-it runs the event count was fitted on: a 3-link card next to one other finishes its
/// chain 89% of the time (observed; the binomial gives 86.6%), next to two others 60%, and alone almost always.
#[test]
fn chain_model_calibration() {
    let m = chain_marginals(&[(3, 3, 0), (3, 3, 0)]);
    assert!((m[&(3, 3, 0)][2] - 0.866).abs() < 0.01, "{:?}", m);
    assert!(m[&(3, 3, 0)][0] > 0.99);
    let m9 = chain_marginals(&[(3, 3, 0), (3, 3, 0), (3, 3, 0)]);
    assert!(m9[&(3, 3, 0)][2] > 0.45 && m9[&(3, 3, 0)][2] < 0.62, "{:?}", m9);
    let one = chain_marginals(&[(3, 3, 0)]);
    assert!(one[&(3, 3, 0)][2] > 0.99);
    // ending a chain early frees its slot: the other card's links fire more often
    let early = chain_marginals(&[(3, 3, 0), (3, 2, 0)]);
    assert!(early[&(3, 3, 0)][2] > m[&(3, 3, 0)][2] && early[&(3, 2, 0)].len() == 2);
    let pm = event_pmf(30);
    assert!(
        (pm.iter().sum::<f64>() - 1.0).abs() < 1e-12
            && (pm.iter().enumerate().map(|(e, p)| e as f64 * p).sum::<f64>() - 7.35).abs() < 1e-9
    );
}
/// A card's training draws over two targets in its pool: each target keeps its own chance 1 - (1 - 1/n)^h, both
/// are hinted together less often than independent chances would say, and the outcomes are a distribution.
#[test]
fn training_draws_over_two_targets() {
    let cr: CardsRaw = fixture_cards();
    let c = cr
        .cards
        .iter()
        .find(|c| c.nskill > 0 && c.whites.contains(&20046) && c.whites.contains(&20161))
        .expect("a card with Ramp Up and Tail Held High");
    let dj = job_with(json!({}));
    let unc = [20046u32, 20161];
    let sb = Sources { dj: &dj, unc: &unc };
    let tr = sb.train(c);
    assert_eq!(tr.len(), 2);
    let g = sb.train_group(c, &tr);
    assert!(g.iter().map(|x| x.0).sum::<f64>() <= 1.0 + 1e-12);
    let gh = group_gh(&g);
    for (k, _, ph) in &gh {
        assert!((ph - tr[*k].1).abs() < 1e-12, "{gh:?} vs {tr:?}");
    }
    let both = g.iter().filter(|x| x.1.len() == 2).map(|x| x.0).sum::<f64>();
    assert!(both < tr[0].1 * tr[1].1);
    let h = hint_base("unity") * (1.0 + c.hf / 100.0) * (1.0 + hint_spec("unity") * c.spec / 100.0);
    assert!((tr[0].1 - (1.0 - hint_miss(1, c.nskill as usize, h, c.hinc))).abs() < 1e-12); // the card's assembled sources carry the same odds (the group is not lost next to its other rewards)
    let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
    src_miss(&sb.src(c, None, 0), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
    for k in 0..2 {
        assert!((1.0 - mw[k] * mg[k] - tr[k].1).abs() < 1e-9 || mg[k] < 1.0, "{mw:?} {mg:?} {tr:?}");
        assert!(mw[k] * mg[k] < 1.0);
    }
}
/// The Lv-5 cap: a skill leaves a card's pool after ceil(5 / levels per draw) draws. Checked against brute-force
/// enumeration of every draw sequence, and on cases worked by hand (a +5 card draws without replacement).
#[test]
fn training_draws_leave_maxed_skills() {
    fn brute(k: usize, n: usize, t: usize, cap: usize, cnt: &mut Vec<usize>) -> f64 {
        if t == 0 {
            return 1.0;
        }
        let avail: Vec<usize> = (0..n).filter(|&i| cnt[i] < cap).collect();
        if avail.is_empty() {
            return 1.0;
        }
        let mut p = 0.0;
        for &i in &avail {
            if i < k {
                continue;
            } // skills 0..k are the given ones: drawing one fails
            cnt[i] += 1;
            p += brute(k, n, t - 1, cap, cnt) / avail.len() as f64;
            cnt[i] -= 1;
        }
        p
    }
    for n in 2..=6 {
        for k in 1..=n {
            for t in 0..=6 {
                for cap in 1..=5 {
                    let b = brute(k, n, t, cap, &mut vec![0; n]);
                    assert!(
                        (miss_draws(k, n, t, cap) - b).abs() < 1e-12,
                        "k {k} n {n} t {t} cap {cap}: {} vs {b}",
                        miss_draws(k, n, t, cap)
                    );
                }
            }
        }
    }
    assert!((miss_draws(1, 5, 4, 1) - 0.2).abs() < 1e-12); // 4 of 5 skills, no repeats
    assert_eq!(miss_draws(2, 5, 4, 1), 0.0); // only 3 others to absorb 4 draws
    assert!((miss_draws(2, 5, 3, 1) - 0.1).abs() < 1e-12); // 1 / C(5, 3)
    assert!((miss_draws(1, 7, 2, 2) - (6.0f64 / 7.0).powi(2)).abs() < 1e-12); // 2 draws, cap 2: no skill leaves before the last
    // fractional h: floor / ceil mixture; levels per draw 1 -> cap 5, 3 or 4 -> 2, 5 -> 1
    assert!((hint_miss(1, 5, 4.25, 5) - (0.75 * 0.2 + 0.25 * 0.0)).abs() < 1e-12);
    assert!((hint_miss(1, 5, 2.0, 1) - 0.64).abs() < 1e-12);
}
/// Stop risk (IT's default choice ends the chain on some outcomes) is the exact mixture over where it stops.
#[test]
fn chain_stop_risk_is_a_mixture() {
    let r12 = stop_risk_code(&[(1, 1, 2), (2, 1, 2)], 3); // ends on 1 of 2 outcomes at links 1 and 2
    let r2 = stop_risk_code(&[(2, 1, 2)], 3);
    assert_eq!(stop_risk_code(&[(2, 1, 2), (3, 1, 2)], 2), 0); // links at or past the stop don't matter
    let full = chain_marginals(&[(3, 3, 0)])[&(3, 3, 0)].clone();
    let alone = chain_marginals(&[(3, 3, r12)])[&(3, 3, r12)].clone();
    assert!(
        (alone[0] - full[0]).abs() < 1e-15
            && (alone[1] - 0.5 * full[1]).abs() < 1e-15
            && (alone[2] - 0.25 * full[2]).abs() < 1e-15,
        "{alone:?}"
    );
    // with a partner: 50% the deck is (full, full), 50% the risky card stops after link 2 and frees its slot
    let mix = chain_marginals(&[(3, 3, 0), (3, 3, r2)]);
    let a = chain_marginals(&[(3, 3, 0), (3, 3, 0)]);
    let b = chain_marginals(&[(3, 3, 0), (3, 2, 0)]);
    for k in 0..3 {
        assert!((mix[&(3, 3, 0)][k] - (0.5 * a[&(3, 3, 0)][k] + 0.5 * b[&(3, 3, 0)][k])).abs() < 1e-15);
    }
    assert!((mix[&(3, 3, r2)][2] - 0.5 * a[&(3, 3, 0)][2]).abs() < 1e-15);
    assert!(mix[&(3, 3, 0)][2] > a[&(3, 3, 0)][2]);
}

/// The game's scenario bans reach the solver with the card data: Team Sirius out of Unity Cup and Grand Concert
/// decks, allowed in URA Finale and Trackblazer.
#[test]
fn scenario_bans_exported() {
    let cr: CardsRaw = fixture_cards();
    for sc in ["unity", "gc"] {
        assert!(cr.restrict.get(sc).is_some_and(|v| v.contains(&30081)), "{sc}: {:?}", cr.restrict);
    }
    for sc in ["ura", "trackblazer"] {
        assert!(!cr.restrict.get(sc).is_some_and(|v| v.contains(&30081)));
    }
}
fn job_with(v: Value) -> DeckJob {
    serde_json::from_value(v).unwrap()
}
fn halo() -> Card {
    let cr: CardsRaw = fixture_cards();
    cr.cards.into_iter().find(|c| c.id == 30056).unwrap()
}
/// An event offering targets on different choices gives one deck entry per choice, each with one choice's
/// rewards; with only one of them a target, the card keeps a single entry with that choice.
#[test]
fn choices_become_variants() {
    let c = halo();
    // King Halo [Tonight, We Waltz], link 1: Stop Right There! (choice 1) or Uma Stan (choice 2)
    let dj = job_with(
        json!({"alts": [[30056, 1, [[[[20102, 1]]], [[[20159, 1]]]]]], "events": [[30056, 20102, 1, 1], [30056, 20159, 1, 1]]}),
    );
    let unc = [20159u32, 20102];
    let sb = Sources { dj: &dj, unc: &unc };
    let vs = sb.variants(&c, None);
    assert_eq!(vs.len(), 2);
    for (i, &v) in vs.iter().enumerate() {
        let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
        src_miss(&sb.src(&c, None, v), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
        // with every link firing, exactly one of the two whites is certain
        let got: Vec<bool> = mw.iter().map(|&x| x < 1e-12).collect();
        assert_eq!(got.iter().filter(|&&x| x).count(), 1, "variant {i}: {mw:?}");
    }
    let one = [20159u32];
    let sb1 = Sources { dj: &dj, unc: &one };
    assert_eq!(sb1.variants(&c, None), vec![0]);
    let (mut mw, mut mg) = (vec![1.0], vec![1.0]);
    src_miss(&sb1.src(&c, None, 0), &[1.0, 1.0, 1.0], &mut mw, &mut mg);
    assert!(mw[0] < 1e-12 && mg[0] == 1.0);
}
/// One event's outcomes exclude each other: a gold on two of four outcomes and its white on the other two give
/// a hint whenever the link fires (independent 50% chances would give 75%).
#[test]
fn outcomes_of_one_event_are_exclusive() {
    let c = halo();
    let dj = job_with(json!({"alts": [[30056, 3, [[[[20203, 2]], [[20203, 1]], [[20203, 2]], [[20203, 1]]]]]]}));
    let unc = [20203u32];
    let sb = Sources { dj: &dj, unc: &unc };
    let (mut mw, mut mg) = (vec![1.0], vec![1.0]);
    src_miss(&sb.src(&c, None, 0), &[1.0, 1.0, 0.8], &mut mw, &mut mg);
    assert!((mg[0] - 0.6).abs() < 1e-12, "{mg:?}"); // P(no gold) = 1 - 0.8 * 0.5
    assert!((mg[0] * mw[0] - 0.2).abs() < 1e-12, "{mw:?}"); // P(no hint) = 1 - 0.8
}
/// Measured outcome shares (`alt_w`) replace equal outcomes: 3:1 between two skills, link firing 80%.
#[test]
fn outcome_shares_weight_alts() {
    let c = halo();
    let dj =
        job_with(json!({"alts": [[30056, 3, [[[[20102, 1]], [[20159, 1]]]]]], "alt_w": [[30056, 3, [[0.75, 0.25]]]]}));
    let unc = [20102u32, 20159];
    let sb = Sources { dj: &dj, unc: &unc };
    let (mut mw, mut mg) = (vec![1.0; 2], vec![1.0; 2]);
    src_miss(&sb.src(&c, None, 0), &[1.0, 1.0, 0.8], &mut mw, &mut mg);
    assert!((mw[0] - (1.0 - 0.8 * 0.75)).abs() < 1e-12 && (mw[1] - (1.0 - 0.8 * 0.25)).abs() < 1e-12, "{mw:?}");
}
/// The final-state distribution has the same per-link marginals as `chain_marginals`, stop risk included.
#[test]
fn chain_states_match_marginals() {
    let r = stop_risk_code(&[(2, 1, 2)], 3);
    for ks in [
        vec![(3, 3, 0)],
        vec![(2, 2, 0), (3, 3, 0), (3, 3, 0)],
        vec![(3, 2, 0), (3, 3, r), (4, 4, 0)],
        vec![(2, 2, 0); 6],
    ] {
        let mut ks: Vec<Kind> = ks;
        ks.sort();
        let st = chain_states(&ks);
        assert!((st.iter().map(|x| x.1).sum::<f64>() - 1.0).abs() < 1e-12);
        let m = chain_marginals(&ks);
        for (i, kd) in ks.iter().enumerate() {
            for l in 1..=kd.1 {
                let p: f64 = st.iter().filter(|x| x.0[i] >= l).map(|x| x.1).sum();
                assert!(
                    (p - m[kd][l as usize - 1]).abs() < 1e-12,
                    "{ks:?} entry {i} link {l}: {p} vs {}",
                    m[kd][l as usize - 1]
                );
            }
        }
    }
}
/// `key_level`'s per-count progress grids project the same process as `chain_states`: each final state's chance is
/// the grids' value at its grid index summed over the event count's distribution (a stop risk and early ends included)
#[test]
fn key_level_matches_states() {
    let r = stop_risk_code(&[(2, 1, 2)], 3);
    let pmf = event_pmf(CHAIN_EVENTS.with(|c| c.get()).0 as usize);
    for ks in [
        vec![(3, 3, 0)],
        vec![(2, 2, 0), (3, 3, 0), (3, 3, 0)],
        vec![(3, 2, 0), (3, 3, r), (4, 4, 0)],
        vec![(2, 2, 0); 6],
    ] {
        let mut ks: Vec<Kind> = ks;
        ks.sort();
        let (st, kl) = (chain_states(&ks), key_level(&ks));
        let at = |gi: usize| (0..kl.lev.len()).map(|m| pmf[m] * kl.lev[m][gi]).sum::<f64>();
        let mut seen = 0.0f64;
        for (s, p) in st.iter() {
            let gi: usize = s.iter().zip(&kl.gstride).map(|(&x, &g)| x as usize * g).sum();
            let q = at(gi);
            seen += q;
            assert!((q - p).abs() < 1e-12, "{ks:?} state {s:?}: key_level {q} vs chain_states {p}");
        }
        // (and no grid mass outside the final states)
        assert!((seen - (0..kl.lev[0].len()).map(at).sum::<f64>()).abs() < 1e-12, "{ks:?}");
    }
}
/// The collapsed-state form equals the full enumeration (chains, coupled outcomes, a stop risk, fillers).
#[test]
fn joint_fast_matches_reference() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
    let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
    // chain cards with single-target and coupled outcomes, a coupled fixed event, a stop risk, fillers
    let a = Src {
        train: vec![(0, 0.3)],
        chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))],
        ..Default::default()
    }
    .finish();
    let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
    let c = Src { train: vec![(2, 0.2)], chains: vec![(1, two(2, 3))], ..Default::default() }.finish();
    let e = Src { fixed: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
    let risk = stop_risk_code(&[(2, 1, 2)], 3);
    let ents = vec![(a, (3, 3, 0)), (b, (2, 2, 0)), (c, (3, 3, risk)), (e, (0, 0, 0))];
    for fill in [0usize, 2] {
        for u in [0usize, 5, 20] {
            let x = joint_fast(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len(), true, None);
            let y = joint_trainee_ref(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len());
            for (p, r) in x.iter().zip(&y) {
                assert!(
                    (p.0 - r.0).abs() < 1e-13 && (p.1 - r.1).abs() < 1e-13 && p.2 == r.2,
                    "fill {fill} uma {u}: {:?} vs {:?}",
                    p,
                    r
                );
            }
            let z = joint_fast(&m, u, &[0.1, 0.5, 0.9], &ents, fill, sj.targets.len(), false, None);
            for (p, r) in z.iter().zip(&y) {
                assert!((p.0 - r.0).abs() < 1e-13);
            }
        }
    }
}

/// Career planner: a per-target inheritance chance folded into the trainee's event hint chances, evaluated at
/// q = 0, gives the same joint odds as the loop model at q; the spark scale multiplies P(all) by the product of
/// the scales; with two targets the exact marginals satisfy P(A) + P(B) = P(all) + P(at least one).
#[test]
fn career_fold_and_scale() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}],
        "trainee_hints": [[1001, 20166, 0.3]]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
    let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
    let a = Src {
        train: vec![(0, 0.3)],
        chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))],
        ..Default::default()
    }
    .finish();
    let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
    let c = Src { train: vec![(2, 0.2)], chains: vec![(1, two(2, 3))], ..Default::default() }.finish();
    let ents = vec![(a, (3, 3, 0)), (b, (2, 2, 0)), (c, (3, 3, 0))];
    let rel = |x: f64, y: f64| (x - y).abs() / y.abs().max(1e-300);
    for u in [0usize, 5, 20] {
        for q in [0.1, 0.6] {
            let x = joint_fast(&m, u, &[q], &ents, 1, sj.targets.len(), true, None)[0];
            let mut f = Model::new(&d, &sk, &sj);
            f.ews[u] = m.ews[u].iter().map(|o| o.iter().map(|e| 1.0 - (1.0 - e) * (1.0 - q)).collect()).collect();
            let y = joint_fast(&f, u, &[0.0], &ents, 1, sj.targets.len(), true, None)[0];
            assert!(
                rel(y.0, x.0) < 1e-12 && rel(y.1, x.1) < 1e-12 && x.2 == y.2,
                "fold uma {u} q {q}: {:?} vs {:?}",
                x,
                y
            );
            let su = vec![1.1f64.powi(-1), 1.0, 1.1f64.powi(-3), 1.1f64.powi(-6)];
            let mut ext =
                JointExt { su: su.clone(), sf: vec![1.1f64.powi(-2)], marg: vec![], sets: vec![], per_outfit: vec![] };
            let z = joint_fast(&f, u, &[0.0], &ents, 1, sj.targets.len(), true, Some(&mut ext))[0];
            let c = su.iter().product::<f64>() * 1.1f64.powi(-2);
            assert!(rel(z.0, c * y.0) < 1e-12 && z.2 == y.2, "scale uma {u}: {} vs {}", z.0, c * y.0);
            assert!(z.1 >= z.0 && z.1 <= 1.0);
        }
    }
    // two targets, coupled sources: marginals against P(all) and P(all but at most one)
    let sj2: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20166}]})).unwrap();
    let m2 = Model::new(&d, &sk, &sj2);
    let p =
        Src { train: vec![(0, 0.2)], chains: vec![(1, two(0, 1)), (2, g1(1, 1, 0.7))], ..Default::default() }.finish();
    let r = Src { fixed: vec![two(1, 0)], chains: vec![(1, g1(0, 2, 0.4))], ..Default::default() }.finish();
    let ents2 = vec![(p, (2, 2, 0)), (r, (2, 2, 0))];
    for u in [0usize, 5, 20] {
        let mut ext = JointExt { su: vec![0.9, 0.8], sf: vec![], marg: vec![], sets: vec![0.0], per_outfit: vec![] };
        let j = joint_fast(&m2, u, &[0.3], &ents2, 0, 2, true, Some(&mut ext))[0];
        let (m0, m1) = (ext.marg[j.2 * 2], ext.marg[j.2 * 2 + 1]);
        assert!(rel(m0 + m1, j.0 + j.1) < 1e-12, "uma {u}: {m0} + {m1} vs {} + {}", j.0, j.1);
        // the landed sets: they sum to 1, {both} is P(all), and each target's sets sum to its marginal
        let st = &ext.sets[(j.2 << 2)..(j.2 << 2) + 4];
        assert!(
            (st.iter().sum::<f64>() - 1.0).abs() < 1e-12
                && rel(st[3], j.0) < 1e-12
                && rel(st[1] + st[3], m0) < 1e-12
                && rel(st[2] + st[3], m1) < 1e-12,
            "sets {:?}",
            st
        );
    }
}

/// The negative-association bound is at least the exact value (chains, a card feeding two targets, a target fed by
/// two cards, early ends, a stop risk, fillers), and close to it.
#[test]
fn upper_bound_covers_exact() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
    let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
    let a = Src {
        train: vec![(0, 0.3)],
        chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))],
        ..Default::default()
    }
    .finish();
    let b = Src { fixed: vec![two(3, 0)], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
    let c = Src { train: vec![(2, 0.2)], chains: vec![(3, g1(2, 2, 0.6))], ..Default::default() }.finish();
    let e = Src { fixed: vec![g1(1, 2, 0.25)], ..Default::default() }.finish();
    let risk = stop_risk_code(&[(2, 1, 2)], 3);
    let decks: Vec<Vec<(Src, Kind)>> = vec![
        vec![(a.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0)), (e.clone(), (0, 0, 0))],
        vec![(a.clone(), (3, 3, 0)), (c.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0))],
        vec![(a.clone(), (3, 2, 0)), (c.clone(), (3, 3, risk)), (b.clone(), (2, 2, 0))],
        vec![(a.clone(), (3, 3, 0)), (c.clone(), (3, 1, 0)), (e.clone(), (0, 0, 0))],
    ];
    let mut covered = 0;
    for ents in &decks {
        for fill in [0usize, 1, 3] {
            for u in [0usize, 5, 20] {
                let qs = [0.1, 0.5, 0.9];
                let x = joint_fast(&m, u, &qs, ents, fill, sj.targets.len(), false, None);
                let er: Vec<(&Src, Kind)> = ents.iter().map(|(s, k)| (s, *k)).collect();
                if let Some(ub) = upper_na(&m, u, &qs, &er, fill) {
                    covered += 1;
                    for (i, y) in x.iter().enumerate() {
                        assert!(
                            ub[i] >= y.0 * (1.0 - 1e-12) && ub[i] <= y.0 * 1.5,
                            "fill {fill} uma {u} q {}: bound {} exact {}",
                            qs[i],
                            ub[i],
                            y.0
                        );
                    }
                }
            }
        }
    }
    assert!(covered >= 30, "bound covered only {covered} cases");
}

/// Groups whose outcomes each hint one target pass the orthant check; an outcome hinting two targets together fails.
#[test]
fn group_orthant_check() {
    assert!(group_nuod(&vec![(0.5, vec![(0, 2)]), (0.3, vec![(1, 1)])]));
    assert!(!group_nuod(&vec![(0.5, vec![(0, 1), (1, 1)]), (0.3, vec![(1, 1)])]));
    assert!(group_nuod(&vec![(0.2, vec![(0, 1), (1, 1)]), (0.3, vec![(0, 1)]), (0.3, vec![(1, 1)])])); // 0.2 <= 0.5 * 0.5
}

/// Exceeding the exhaustive-check budget must not certify a positively correlated group, or shift past u32.
#[test]
fn group_orthant_check_declines_unchecked_groups() {
    for n in [20, 21, 32, 33] {
        // Any pair has joint probability 0.5, above the product of its marginals (0.25).
        let g = vec![(0.5, (0..n).map(|k| (k, 1)).collect())];
        assert!(!group_nuod(&g), "{n} targets");
        for src in
            [Src { fixed: vec![g.clone()], ..Default::default() }, Src { chains: vec![(1, g)], ..Default::default() }]
        {
            assert!(src.finish().not_nuod, "{n} targets");
        }
    }
}

/// Inline event alternatives can reach the unchecked-group case without exponential training-hint setup.
#[test]
fn unchecked_inline_event_disables_orthant_bound() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let unc: Vec<u32> = sk.skills.iter().take(21).map(|s| s.0).collect();
    assert_eq!(unc.len(), 21);
    let targets: Vec<Value> = unc.iter().map(|g| json!({"group": g})).collect();
    let sj: SkillJob = serde_json::from_value(json!({"targets": targets})).unwrap();
    let model = Model::new(&d, &sk, &sj);
    let mut card = halo();
    card.nskill = 0;
    card.whites.clear();
    let rewards: Vec<(u32, u8)> = unc.iter().map(|&g| (g, 1)).collect();
    let dj = job_with(json!({"alts": [[card.id, 1, [[rewards, []]]]]}));
    let sb = Sources { dj: &dj, unc: &unc };
    let src = sb.src(&card, None, 0);
    assert_eq!(src.chains.len(), 1);
    assert_eq!(src.chains[0].1[0].0, 0.5);
    assert_eq!(src.chains[0].1[0].1.len(), 21);
    assert!(src.not_nuod);
    assert!(upper_na(&model, 0, &[0.0], &[(&src, sb.kind(&card, None))], 0).is_none());
}

/// The level check behind the cap-free bound (`group_nuod_lvl`): single-target outcomes pass, an outcome hinting two
/// targets together fails; gold on one target with white on another in one outcome fails where the any-hint check
/// (`group_nuod`) passes (P(gold 0, 1) = 0.2 > 0.2 x 0.5); and for decks of covered groups the state value is at
/// least the exact value.
#[test]
fn group_level_orthant_check() {
    assert!(group_nuod_lvl(&vec![(0.5, vec![(0, 2)]), (0.3, vec![(1, 1)])]));
    assert!(!group_nuod_lvl(&vec![(0.5, vec![(0, 1), (1, 1)]), (0.3, vec![(1, 1)])]));
    let g = vec![(0.2, vec![(0, 2), (1, 1)]), (0.3, vec![(0, 1)]), (0.3, vec![(1, 1)])];
    assert!(group_nuod(&g) && !group_nuod_lvl(&g));
    // three targets, white only, joint terms below the products (a card's training draws)
    assert!(group_nuod_lvl(&vec![
        (0.2, vec![(0, 1)]),
        (0.2, vec![(1, 1)]),
        (0.2, vec![(2, 1)]),
        (0.03, vec![(0, 1), (1, 1)]),
        (0.03, vec![(1, 1), (2, 1)]),
        (0.03, vec![(0, 1), (2, 1)])
    ]));
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
    let two = |a: usize, b: usize| -> Group { vec![(0.5, vec![(a, 2)]), (0.3, vec![(b, 1)])] };
    // (an outcome hinting two targets at once, at most independent at every level: covered, though not `group_assoc`)
    let three: Group =
        vec![(0.3, vec![(0, 1)]), (0.3, vec![(2, 1)]), (0.3, vec![(3, 1)]), (0.02, vec![(0, 1), (2, 1)])];
    let a = Src {
        train: vec![(0, 0.3)],
        chains: vec![(1, g1(1, 1, 1.0)), (2, two(1, 2)), (3, g1(0, 2, 0.5))],
        ..Default::default()
    }
    .finish();
    let b = Src { fixed: vec![two(3, 0), three], chains: vec![(2, g1(3, 2, 1.0))], ..Default::default() }.finish();
    let c = Src { train: vec![(2, 0.2)], chains: vec![(3, g1(2, 2, 0.6))], ..Default::default() }.finish();
    assert!(a.cf_ok && b.cf_ok && c.cf_ok && b.multi);
    for ents in [
        vec![(a.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0))],
        vec![(a.clone(), (3, 3, 0)), (c.clone(), (3, 3, 0)), (b.clone(), (2, 2, 0))],
    ] {
        for fill in [0usize, 2] {
            for u in [0usize, 5, 20] {
                let qs = [0.1, 0.5, 0.9];
                let x = joint_fast(&m, u, &qs, &ents, fill, sj.targets.len(), false, None);
                let er: Vec<(&Src, Kind)> = ents.iter().map(|(s, k)| (s, *k)).collect();
                let sv = state_value_cf(&m, u, &qs, &er, fill).unwrap();
                for (i, y) in x.iter().enumerate() {
                    assert!(sv[i] >= y.0 * (1.0 - 1e-12), "fill {fill} uma {u}: state value {} exact {}", sv[i], y.0);
                }
            }
        }
    }
}

/// With no chain cards and no event touching two targets, every source is independent: the exact value equals
/// the product of per-target odds the search ranks by.
#[test]
fn joint_equals_product_without_correlation() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json"));
    let sk: SkillsRaw = fixture_skills();
    let sj: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20049,"guaranteed":2},{"group":20166},{"group":20161},{"group":20203}]})).unwrap();
    let m = Model::new(&d, &sk, &sj);
    let nu = m.uncertain.len();
    let g1 = |k: usize, lvl: u8, p: f64| -> Group { vec![(p, vec![(k, lvl)])] };
    let a = Src {
        train: vec![(0, 0.3), (2, 0.2)],
        fixed: vec![g1(1, 2, 0.5), g1(3, 2, 0.5), g1(3, 1, 1.0)],
        ..Default::default()
    }
    .finish();
    let b = Src { train: vec![(1, 0.4)], fixed: vec![g1(0, 2, 0.25)], ..Default::default() }.finish();
    let (mut mw, mut mg) = (vec![1.0; nu], vec![1.0; nu]);
    for x in [&a, &b] {
        src_miss(x, &[], &mut mw, &mut mg);
    }
    let dw: Vec<f64> = mw.iter().map(|x| 1.0 - x).collect();
    let dg: Vec<f64> = mg.iter().map(|x| 1.0 - x).collect();
    for u in [0usize, 5, 20] {
        for q in [0.3, 0.75] {
            let j = joint(&m, u, &[q], &[(a.clone(), (0, 0, 0)), (b.clone(), (0, 0, 0))], 0, sj.targets.len(), true);
            let p = m.fixed_factor() * m.trainee_fast(u, q, &dw, &dg);
            assert!((j[0].0 - p).abs() < 1e-14, "uma {u} q {q}: {} vs {p}", j[0].0);
        }
    }
}
/// `top_asc` and `top_desc` keep the stable sorts' first k values bit for bit, ties (0 and -0 among them) in input order.
#[test]
fn top_k_is_stable_sort_prefix() {
    let pool = [0.0f64, -0.0, 0.25, 0.5, 1.0, 0.5, f64::INFINITY, 1e-300, 0.25];
    let mut s = 12345u64;
    for _ in 0..4000 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let n = (s >> 60) as usize;
        let k = ((s >> 56) & 7) as usize;
        let v: Vec<f64> = (0..n).map(|i| pool[((s >> (i * 4 % 48)) % pool.len() as u64) as usize]).collect();
        let (mut a, mut d) = (vec![], vec![]);
        for &x in &v {
            top_asc(&mut a, k, x);
            top_desc(&mut d, k, x);
        }
        let mut sa = v.clone();
        sa.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let mut sd = v.clone();
        sd.sort_by(|x, y| y.partial_cmp(x).unwrap());
        let bits = |x: &[f64]| x.iter().map(|y| y.to_bits()).collect::<Vec<_>>();
        assert_eq!(bits(&a), bits(&sa[..k.min(n)]), "{v:?} k {k}");
        assert_eq!(bits(&d), bits(&sd[..k.min(n)]), "{v:?} k {k}");
    }
}
/// The ng/nh landing factor's forms agree to the bit: `land_factor`, `LandPre::f` (its constants taken once) and the
/// expression `joint_eval` and `upper_na` wrote out before they called it, on random and non-finite inputs
#[test]
fn land_factor_forms_match() {
    use crate::skills::GEN_WHITE;
    let reference = |st: u8, ew: f64, q: f64, ng: f64, nh: f64| {
        let pg = 1.0 - ng;
        match st {
            2 => GEN_GOLD,
            1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - nh * (1.0 - ew) * (1.0 - q)),
        }
    };
    let mut s = 0x2545f4914f6cdd1du64;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    let odd = [f64::NAN, f64::INFINITY, -0.0, 0.0, 1.0, 1e-300];
    for it in 0..20000usize {
        let st = (rnd() * 3.0) as u8;
        let mut f = |i: usize| if it % 5 == 0 && rnd() < 0.3 { odd[(it + i) % odd.len()] } else { rnd() };
        let (ew, q, ng, nh) = (f(0), f(1), f(2), f(3));
        for (g, h) in [(ng, nh), (ng, 0.0), (0.0, 0.0)] {
            let r = reference(st, ew, q, g, h).to_bits();
            assert_eq!(land_factor(st, ew, q, g, h).to_bits(), r, "{st} {ew} {q} {g} {h}");
            assert_eq!(LandPre::new(st, ew, q).f(g, h).to_bits(), r, "{st} {ew} {q} {g} {h}");
        }
    }
}
