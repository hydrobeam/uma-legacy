//! Deck dominance: signatures, buckets and the beaten-deck bookkeeping (`Kept`), and the decks a search returns.
use super::*;

/// Slack for float noise in dominance tests: per coordinate, and summed over all coordinates.
pub(super) const DOM_EPS: f64 = 1e-12;
pub(super) const SUM_EPS: f64 = 1e-9;

#[inline]
pub(super) fn dominates(by: &[f64], x: &[f64]) -> bool {
    by.iter().zip(x).all(|(b, a)| *b >= *a - DOM_EPS)
}

/// Coarse signature for a fast necessary test of dominance: the first SIG_FIELDS coordinates as levels
/// floor(LEVELS * v) (capped at LEVELS - 1), each in a field of FB bits whose top bit is a guard. `entry`
/// signatures use the value itself, `query` ones the value less DOM_EPS, so `by` dominating `x` implies every
/// field of entry(by) >= the matching field of query(x). A 64-bit word holds 16 fields of 8 levels, enough for
/// 8 targets x (gold, any hint), and is cheap in WebAssembly as well as natively.
pub(super) type Sig = u64;
pub(super) const FB: usize = 4;
pub(super) const SIG_FIELDS: usize = 64 / FB;
pub(super) const LEVELS: Sig = 1 << (FB - 1);
pub(super) const SIG_GUARD: Sig = {
    let mut g: Sig = 0;
    let mut c = 0;
    while c < SIG_FIELDS {
        g |= (1 as Sig) << (FB * c + FB - 1);
        c += 1;
    }
    g
};
pub(super) fn sig_of(v: &[f64], query: bool) -> Sig {
    let mut s: Sig = 0;
    for (c, &x) in v.iter().take(SIG_FIELDS).enumerate() {
        let y = if query { x - DOM_EPS } else { x };
        let l = (((y * LEVELS as f64).floor().max(0.0) as u32) as Sig).min(LEVELS - 1); // via u32: wasm2js lacks the 64-bit float conversion
        s |= l << (FB * c);
    }
    s
}
/// every field of `e` >= the matching field of `q` (SWAR: guard bits survive the subtraction)
#[inline]
pub(super) fn sig_ge(e: Sig, q: Sig) -> bool {
    ((e | SIG_GUARD) - q) & SIG_GUARD == SIG_GUARD
}

pub(super) const NBUCKET: usize = 128;

/// Decks whose coordinate sums fall in one slice of [0, nd]: flat arrays (entry and query signatures,
/// coordinates, ids) plus a level index of the entry signatures: for each word of 64 decks and each signature
/// field f and level l >= 1, the bits of the decks whose field f is at least l. ANDing one word per field of a
/// query gives the decks that pass the signature test, 64 at a time; they are then visited in index order, the
/// same order as a plain scan.
#[derive(Default)]
/// `lq` is the mirror index of the query signatures (bit set when field f is at most l, l < LEVELS - 1), for the
/// removal test "x's entry signature over the deck's query one".
pub(super) struct Bucket {
    // These columns share deck indices. Compaction must move every column together.
    pub(super) list: Vec<DeckEval>,
    pub(super) esig: Vec<Sig>,
    pub(super) qsig: Vec<Sig>,
    pub(super) flat: Vec<f64>,
    pub(super) uid: Vec<u64>,
    pub(super) lv: Vec<u64>,
    pub(super) lq: Vec<u64>,
}
pub(super) const LVW: usize = SIG_FIELDS * (LEVELS as usize - 1); // index words per 64 decks
impl Bucket {
    #[inline]
    pub(super) fn index_set(&mut self, i: usize, e: Sig, q: Sig) {
        let w = i / 64;
        if self.lv.len() < (w + 1) * LVW {
            self.lv.resize((w + 1) * LVW, 0);
            self.lq.resize((w + 1) * LVW, 0);
        }
        let bit = 1u64 << (i % 64);
        let base = w * LVW;
        const L1: usize = LEVELS as usize - 1;
        for f in 0..SIG_FIELDS {
            let l = ((e >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in 0..l {
                self.lv[base + f * L1 + k] |= bit;
            }
            let l = ((q >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in l..L1 {
                self.lq[base + f * L1 + k] |= bit;
            }
        }
    }
    /// rebuild the level index from deck `from` on (after the arrays were compacted)
    pub(super) fn reindex_from(&mut self, from: usize) {
        let first_word = from / 64;
        self.lv.truncate((first_word + 1) * LVW);
        self.lq.truncate((first_word + 1) * LVW);
        if self.lv.len() == (first_word + 1) * LVW {
            let keep_bits = if from.is_multiple_of(64) { 0 } else { u64::MAX >> (64 - from % 64) };
            for word in &mut self.lv[first_word * LVW..] {
                *word &= keep_bits;
            }
            for word in &mut self.lq[first_word * LVW..] {
                *word &= keep_bits;
            }
        }
        if self.esig.len() <= first_word * 64 {
            self.lv.truncate(first_word * LVW);
            self.lq.truncate(first_word * LVW);
        }
        for deck in from..self.esig.len() {
            let (entry_sig, query_sig) = (self.esig[deck], self.qsig[deck]);
            self.index_set(deck, entry_sig, query_sig);
        }
    }
    /// Deck bits in `word` that pass all of the query's level-index tests.
    #[inline]
    pub(super) fn cand_word(&self, word: usize, query_offsets: &[usize]) -> u64 {
        let remaining = self.esig.len() - word * 64;
        let mut candidates = if remaining >= 64 { u64::MAX } else { (1u64 << remaining) - 1 };
        let base = word * LVW;
        for &offset in query_offsets {
            candidates &= self.lv[base + offset];
            if candidates == 0 {
                break;
            }
        }
        candidates
    }
}
/// Level-index offsets of a query signature: one per field whose level is at least 1.
pub(super) fn query_offsets(q: Sig, out: &mut [usize; SIG_FIELDS]) -> usize {
    let mut n = 0;
    for f in 0..SIG_FIELDS {
        let l = ((q >> (FB * f)) & (LEVELS - 1)) as usize;
        if l > 0 {
            out[n] = f * (LEVELS as usize - 1) + l - 1;
            n += 1;
        }
    }
    n
}

/// Kept decks of one pal count. A dominator's coordinate sum is at least the deck's (less float slack), so a
/// query only visits buckets at or above the deck's sum.
/// `recent`: coordinates of the last RECENT decks that dominated a new deck.
///   top_decks = 1: a deck on it may since have been dropped, but then a kept deck dominates it, so it still
///   proves that any deck it dominates is not needed.
///   top_decks > 1: kept decks are only dropped at compaction, which starts a fresh `Kept`, so every deck on it
///   is still kept; `ruid` holds their ids so each counts once.
pub(super) struct Kept {
    pub(super) nd: usize,
    pub(super) bw: f64,
    pub(super) buckets: Vec<Bucket>,
    pub(super) n: usize,
    pub(super) next_uid: u64,
    pub(super) lastrow: Vec<f64>,
    pub(super) has_last: bool,
    pub(super) cap: usize,
    pub(super) recent: Vec<f64>,
    pub(super) rpos: usize,
    pub(super) rlen: usize,
    pub(super) ruid: Vec<u64>,
    pub(super) found: Vec<(usize, usize)>,
}
pub(super) const RECENT: usize = 32;
impl Kept {
    pub(super) fn new(nd: usize) -> Self {
        Kept {
            nd,
            bw: (nd.max(1) as f64) / NBUCKET as f64,
            buckets: (0..NBUCKET).map(|_| Bucket::default()).collect(),
            n: 0,
            next_uid: 0,
            lastrow: vec![0.0; nd],
            has_last: false,
            cap: 512,
            recent: vec![0.0; RECENT * nd],
            rpos: 0,
            rlen: 0,
            ruid: vec![u64::MAX; RECENT],
            found: vec![],
        }
    }
    pub(super) fn len(&self) -> usize {
        self.n
    }
    #[inline]
    fn bucket_of(&self, s: f64) -> usize {
        ((s / self.bw).floor().max(0.0) as usize).min(NBUCKET - 1)
    }
    /// number of kept decks dominating x (stops at `need`)
    pub(super) fn count(&mut self, x: &[f64], xq: Sig, s: f64, need: usize, tests: &mut usize) -> usize {
        let nd = self.nd;
        let b0 = self.bucket_of(s - SUM_EPS);
        let mut n = 0;
        self.found.clear();
        let mut qo = [0usize; SIG_FIELDS];
        let nq = query_offsets(xq, &mut qo);
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for w in 0..bk.esig.len().div_ceil(64) {
                let mut cm = bk.cand_word(w, &qo[..nq]);
                while cm != 0 {
                    let i = w * 64 + cm.trailing_zeros() as usize;
                    cm &= cm - 1;
                    *tests += 1;
                    let row = &bk.flat[i * nd..(i + 1) * nd];
                    if dominates(row, x) {
                        n += 1;
                        self.found.push((bi, i));
                        if n >= need {
                            self.lastrow.copy_from_slice(row);
                            self.has_last = true;
                            return n;
                        }
                    }
                }
            }
        }
        n
    }
    /// like `count`, without touching the remembered-dominator state (for a fixed reference set)
    pub(super) fn count_ro(&self, x: &[f64], xq: Sig, s: f64, need: usize) -> usize {
        let nd = self.nd;
        let b0 = self.bucket_of(s - SUM_EPS);
        let mut n = 0;
        let mut qo = [0usize; SIG_FIELDS];
        let nq = query_offsets(xq, &mut qo);
        for bi in (b0..NBUCKET).rev() {
            let bk = &self.buckets[bi];
            for w in 0..bk.esig.len().div_ceil(64) {
                let mut cm = bk.cand_word(w, &qo[..nq]);
                while cm != 0 {
                    let i = w * 64 + cm.trailing_zeros() as usize;
                    cm &= cm - 1;
                    if dominates(&bk.flat[i * nd..(i + 1) * nd], x) {
                        n += 1;
                        if n >= need {
                            return n;
                        }
                    }
                }
            }
        }
        n
    }
    pub(super) fn count_recent(&self, x: &[f64]) -> usize {
        let nd = self.nd;
        (0..self.rlen).filter(|&r| dominates(&self.recent[r * nd..(r + 1) * nd], x)).count()
    }
    /// put the dominators the last count found on `recent`
    pub(super) fn remember_found(&mut self) {
        let nd = self.nd;
        // (the buffer put back after, its capacity kept for the next `count`)
        let mut found = std::mem::take(&mut self.found);
        for &(bi, i) in &found {
            let id = self.buckets[bi].uid[i];
            if self.ruid[..self.rlen].contains(&id) {
                continue;
            }
            let r = self.rpos;
            let (rec, flat) = (&mut self.recent, &self.buckets[bi].flat);
            rec[r * nd..(r + 1) * nd].copy_from_slice(&flat[i * nd..(i + 1) * nd]);
            self.ruid[r] = id;
            self.rpos = (r + 1) % RECENT;
            self.rlen = (self.rlen + 1).min(RECENT);
        }
        found.clear();
        self.found = found;
    }
    pub(super) fn insert(&mut self, x: DeckEval) {
        self.insert_at(x, None)
    }
    /// `insert` with the deck's coordinates given apart (`x.dv` then left as it is: the enumeration's single-result
    /// path, where nothing reads a kept deck's `dv` again, leaves it empty)
    pub(super) fn insert_dv(&mut self, x: DeckEval, dv: &[f64]) {
        self.insert_at(x, Some(dv))
    }
    fn insert_at(&mut self, x: DeckEval, dv: Option<&[f64]>) {
        let bi = self.bucket_of(x.s);
        let id = self.next_uid;
        self.next_uid += 1;
        let bk = &mut self.buckets[bi];
        let i = bk.list.len();
        {
            let dv = dv.unwrap_or(&x.dv);
            let (e, q) = (sig_of(dv, false), sig_of(dv, true));
            bk.index_set(i, e, q);
            bk.esig.push(e);
            bk.qsig.push(q);
            bk.flat.extend_from_slice(dv);
        }
        bk.uid.push(id);
        bk.list.push(x);
        self.n += 1;
    }
    /// drop kept decks that x dominates (only decks with sum <= s + slack can be), in place, keeping the order of
    /// the rest
    pub(super) fn remove_dominated_by(&mut self, x: &[f64], xe: Sig, s: f64) {
        let nd = self.nd;
        let top = self.bucket_of(s + SUM_EPS);
        // lq offsets of x's entry signature: fields below the top level constrain the deck's query field
        let mut xo = [0usize; SIG_FIELDS];
        let mut nx = 0;
        for f in 0..SIG_FIELDS {
            let l = ((xe >> (FB * f)) & (LEVELS - 1)) as usize;
            if l < LEVELS as usize - 1 {
                xo[nx] = f * (LEVELS as usize - 1) + l;
                nx += 1;
            }
        }
        for bi in 0..=top {
            let bk = &mut self.buckets[bi];
            let len = bk.qsig.len();
            // the first deck x dominates: entry test (x's entry signature over the deck's query one) first
            let mut first = None;
            'scan: for start in (0..len).step_by(64) {
                let n = len - start;
                let mut m = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
                let base = start / 64 * LVW;
                for &o in &xo[..nx] {
                    if m == 0 {
                        break;
                    }
                    m &= bk.lq[base + o];
                }
                while m != 0 {
                    let i = start + m.trailing_zeros() as usize;
                    m &= m - 1;
                    if dominates(x, &bk.flat[i * nd..(i + 1) * nd]) {
                        first = Some(i);
                        break 'scan;
                    }
                }
            }
            let Some(first_removed) = first else { continue };
            // Compact from the first removed deck, preserving survivor order. Dropped decks
            // end up at or beyond write_index and are truncated after all columns move.
            let mut write_index = first_removed;
            for read_index in first_removed + 1..len {
                if sig_ge(xe, bk.qsig[read_index]) && dominates(x, &bk.flat[read_index * nd..(read_index + 1) * nd]) {
                    continue;
                }
                bk.list.swap(write_index, read_index);
                bk.esig[write_index] = bk.esig[read_index];
                bk.qsig[write_index] = bk.qsig[read_index];
                bk.uid[write_index] = bk.uid[read_index];
                bk.flat.copy_within(read_index * nd..(read_index + 1) * nd, write_index * nd);
                write_index += 1;
            }
            self.n -= len - write_index;
            bk.list.truncate(write_index);
            bk.esig.truncate(write_index);
            bk.qsig.truncate(write_index);
            bk.uid.truncate(write_index);
            bk.flat.truncate(write_index * nd);
            bk.reindex_from(first_removed);
        }
    }
    pub(super) fn into_list(self) -> Vec<DeckEval> {
        self.buckets.into_iter().flat_map(|b| b.list).collect()
    }
}

/// The order k_filter(list, 1, nd).into_list() produces when it keeps every deck: stable by
/// (bucket of the sum ascending, sum descending), ties in input order.
pub(super) fn kfilter_order(mut g: Vec<DeckEval>, nd: usize) -> Vec<DeckEval> {
    g.sort_by(|a, b| b.s.partial_cmp(&a.s).unwrap());
    let kept = Kept::new(nd);
    g.sort_by_key(|x| kept.bucket_of(x.s));
    g
}
/// Keep each deck unless `kmax` decks scoring at least as high in every coordinate are kept before it (sorted
/// pass). A dominator's coordinate sum is at least the deck's, so only kept decks above that sum are checked.
pub(super) fn k_filter(mut g: Vec<DeckEval>, kmax: usize, nd: usize) -> Kept {
    g.sort_by(|a, b| b.s.partial_cmp(&a.s).unwrap());
    let mut kept = Kept::new(nd);
    let mut tests = 0usize;
    for x in g {
        let xq = sig_of(&x.dv, true);
        if kept.count(&x.dv, xq, x.s, kmax, &mut tests) < kmax {
            kept.insert(x);
        }
    }
    kept.has_last = false;
    kept
}

/// One candidate deck. `dw`/`dg` are the per-target white/gold hint odds the metric uses; `dv` are the
/// coordinates used for dominance (see `run_deck`), all higher-is-better, and `s` is their sum.
#[derive(Clone)]
pub(super) struct DeckEval {
    pub(super) pals: usize,
    pub(super) borrowed: Option<usize>,
    pub(super) cards: Vec<usize>,
    pub(super) ends: Vec<u8>,
    pub(super) vars: Vec<u32>,
    pub(super) plain: usize,
    pub(super) filler: usize,
    pub(super) dw: Vec<f64>,
    pub(super) dg: Vec<f64>,
    pub(super) dv: Vec<f64>,
    pub(super) s: f64,
}

/// A deck's per-target (white, gold) hint odds.
pub(super) trait Odds {
    fn dw(&self) -> &[f64];
    fn dg(&self) -> &[f64];
}
impl Odds for DeckEval {
    fn dw(&self) -> &[f64] {
        &self.dw
    }
    fn dg(&self) -> &[f64] {
        &self.dg
    }
}
impl Odds for PDeck {
    fn dw(&self) -> &[f64] {
        &self.dw
    }
    fn dg(&self) -> &[f64] {
        &self.dg
    }
}
/// A cluster of decks (indices into the list it was built from) with their componentwise maximum odds.
pub(super) struct Cl {
    pub(super) mdw: Vec<f64>,
    pub(super) mdg: Vec<f64>,
    pub(super) mem: Vec<usize>,
}
/// Clusters of at most 16 decks by recursive median splits on the widest odds coordinate.
pub(super) fn split<D: Odds>(sky: &[D], ix: &mut [usize], nu: usize, out: &mut Vec<Cl>) {
    if ix.len() <= 16 {
        let mut mdw = vec![0.0f64; nu];
        let mut mdg = vec![0.0f64; nu];
        for &j in ix.iter() {
            for k in 0..nu {
                mdw[k] = mdw[k].max(sky[j].dw()[k]);
                mdg[k] = mdg[k].max(sky[j].dg()[k]);
            }
        }
        out.push(Cl { mdw, mdg, mem: ix.to_vec() });
        return;
    }
    let coord = |j: usize, c: usize| if c < nu { sky[j].dw()[c] } else { sky[j].dg()[c - nu] };
    let mut lo = vec![f64::INFINITY; 2 * nu];
    let mut hi = vec![f64::NEG_INFINITY; 2 * nu];
    for &j in ix.iter() {
        for k in 0..nu {
            lo[k] = lo[k].min(sky[j].dw()[k]);
            hi[k] = hi[k].max(sky[j].dw()[k]);
            lo[nu + k] = lo[nu + k].min(sky[j].dg()[k]);
            hi[nu + k] = hi[nu + k].max(sky[j].dg()[k]);
        }
    }
    let c = (0..2 * nu).max_by(|&a, &b| (hi[a] - lo[a]).partial_cmp(&(hi[b] - lo[b])).unwrap()).unwrap();
    let mid = ix.len() / 2;
    ix.select_nth_unstable_by(mid, |&a, &b| coord(a, c).partial_cmp(&coord(b, c)).unwrap());
    let (l, r) = ix.split_at_mut(mid);
    split(sky, l, nu, out);
    split(sky, r, nu, out);
}

/// A deck detached from the search that built it (names and ids instead of indices), for per-uma mode. Its names are
/// shared strings (`Name`: a skyline of 50k decks repeats about a hundred distinct ones), written as plain strings
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct PDeck {
    pub(super) names: Vec<Name>,
    pub(super) ids: Vec<u32>,
    pub(super) ends: Vec<u8>,
    #[serde(default)]
    pub(super) vars: Vec<u32>,
    #[serde(default)]
    pub(super) choices: Vec<Name>,
    pub(super) plain_ids: Vec<u32>,
    pub(super) pal_names: Vec<Name>,
    pub(super) pals: usize,
    pub(super) borrowed: Option<Name>,
    pub(super) dw: Vec<f64>,
    pub(super) dg: Vec<f64>,
}
/// A card or deck name shared between decks (serde's `rc`: serialised as the string)
pub(super) type Name = std::rc::Rc<str>;
impl PDeck {
    pub(super) fn out(&self, unc: &[u32]) -> Value {
        let odds: Vec<Value> =
            (0..unc.len()).map(|k| json!({"group": unc[k], "white": self.dw[k], "gold": self.dg[k]})).collect();
        json!({"pals": self.pals, "cards": self.names, "odds": odds, "pal_cards": self.pal_names, "borrowed": self.borrowed,
               "card_ids": self.ids, "end_chain_at": self.ends, "choices": self.choices, "plain_ids": self.plain_ids})
    }
    pub(super) fn blocks(&self, uma: u32, card_of: &crate::FxMap<u32, &Card>) -> bool {
        self.ids.iter().chain(&self.plain_ids).any(|id| card_of.get(id).is_some_and(|c| c.blocks(uma)))
    }
    /// the deck's identity (cards, end links, variants, plain pals) as a fixed key: at most `DECK_ID_N` cards (a deck
    /// holds at most 6: the key codes count kinds in base 7), the unused places 0
    pub(super) fn id(&self) -> DeckId {
        let n = self.ids.len();
        assert!(
            n <= DECK_ID_N && self.ends.len() <= DECK_ID_N && self.vars.len() <= DECK_ID_N,
            "a deck of more than {DECK_ID_N} cards"
        );
        let mut k = DeckId {
            n: n as u8,
            ne: self.ends.len() as u8,
            nv: self.vars.len() as u8,
            ids: [0; DECK_ID_N],
            ends: [0; DECK_ID_N],
            vars: [0; DECK_ID_N],
            plain: self.plain_ids.len(),
        };
        k.ids[..n].copy_from_slice(&self.ids);
        k.ends[..self.ends.len()].copy_from_slice(&self.ends);
        k.vars[..self.vars.len()].copy_from_slice(&self.vars);
        k
    }
}
/// `PDeck::id`'s key: equal exactly when the (ids, ends, vars, plain pal count) are (the lengths kept)
pub(super) const DECK_ID_N: usize = 8;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct DeckId {
    n: u8,
    ne: u8,
    nv: u8,
    ids: [u32; DECK_ID_N],
    ends: [u8; DECK_ID_N],
    vars: [u32; DECK_ID_N],
    plain: usize,
}

/// Order for exact ties between deck entries: card order, except that one character's entries take her positions in
/// order of rarity (highest first; stable), so an SSR wins an exact tie with her own R card ([Planned Perfection] over
/// [Tracen Academy] Riko Kashimoto, the same event) and ties between different characters keep card order.
pub(super) fn tie_rank(rel: &[usize], cards: &[&Card]) -> Vec<usize> {
    let mut rank: Vec<usize> = (0..rel.len()).collect();
    let mut by: Vec<(u32, Vec<usize>)> = vec![];
    for (j, &r) in rel.iter().enumerate() {
        match by.iter_mut().find(|x| x.0 == cards[r].chara) {
            Some(x) => x.1.push(j),
            None => by.push((cards[r].chara, vec![j])),
        }
    }
    for (_, js) in &by {
        let mut s = js.clone();
        s.sort_by_key(|&j| std::cmp::Reverse(cards[rel[j]].rarity));
        for (&pos, &j) in js.iter().zip(&s) {
            rank[j] = pos;
        }
    }
    rank
}
