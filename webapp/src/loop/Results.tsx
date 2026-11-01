// ---------- results: one card per loop (odds, members, decks), the odds, schedule and affinity buttons ----------
// `Results` draws state.ts's `loopDrawn` into #results: each result keyed by its loop and decks (keys.ts), so a redraw with the same
// loops (an interim answer, then the final) keeps every element, an open (i) panel and the focus. A card or a member no
// longer owned is greyed (`off`) from the settings as they change. Their panels are popover.ts's: it reads what it
// shows from the buttons (data-*), and each button draws its aria-expanded from popover.ts's `popOpener`.
// `ResultsCount` is the Results 1 or 3 switch above them.
import { batch, untracked } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { $, fmtIn } from '../dom.ts';
import { gameData } from '../data.ts';
import { setRunnerFrom } from '../images.ts';
import { borLast, cardTitle, chainEndNote } from '../cards.ts';
import { closePop, popHasFocus, popOpener } from '../popover.ts';
import { outfitNote, pickOutfit, umaState } from '../roster.ts';
import { openSched } from '../ui/Schedule.tsx';
import { deckHint } from '../odds.ts';
import { openOdds } from '../ui/OddsDialog.tsx';
import { fields, setDecksTop } from '../settings.ts';
import { RadioPills } from '../ui/RadioPills.tsx';
import { CardAv, SkillIcon, UmaAv } from '../ui/Images.tsx';
import { openWhy } from './Affinity.tsx';
import type { JobTarget } from './job.ts';
import { deckKey, loopKey, resultKeys } from './keys.ts';
import { draw, loopDrawn } from './state.ts';
import type { LoopReport, LoopResult, ResultDeck, ResultLoop } from './types.ts';

const SHOWN = 0.005; // hint odds and their changes under half a percent are not shown
type Odds = ResultDeck['odds'][number];
// the chance of at least the white: a gold hint counts for the white too
const atLeastWhite = (odds: Odds | undefined) => (odds ? deckHint(odds.gold, odds.white) : 0);
const namesOf = (group: number) => gameData().skillNames.get(group) || { w: String(group), gd: null };
// the deck's card indices among `indices`, the borrowed card last (the sixth slot, as in the game)
const borrowedLast = (deck: ResultDeck, indices: number[]) =>
  borLast(deck.cards).filter((i: number) => indices.includes(i));
const allIndices = (list: readonly unknown[]) => list.map((_, i) => i);
const percent = (p: number) => `${Math.round(p * 100)}%`;

// a button that opens a popover.ts panel: its aria-expanded says whether the panel is open from it
function useOpener() {
  const self = useRef<HTMLButtonElement>(null),
    open = popOpener.value;
  return { ref: self, 'aria-expanded': open !== null && open === self.current };
}
/** draws a result (run.ts's `render`): the panel open on the old one closes, the runner becomes one of its umas. Focus
 *  inside the old result (an open panel's Owned switch, say) is noted in the drawn record (`refocus`), so that once the
 *  redraw is done focus that it removed goes to the results column, not the page top */
export function drawResult(r: LoopReport, targets: readonly JobTarget[]) {
  const refocus = popHasFocus() || $('results').contains(document.activeElement);
  batch(() => {
    closePop();
    draw(r, targets, refocus);
  });
  setRunnerFrom(r.top[0].loop.members.map((m) => m.id)); // the runner while the next search works: one of this loop's umas
}

/** every result drawn, one card each, ranked when there are more than one */
export function Results() {
  const drawn = loopDrawn.value;
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per drawn record (two in a row may each refocus)
  useLayoutEffect(() => {
    if (drawn?.refocus && !document.activeElement?.closest('#results')) $('rcol').focus({ preventScroll: true });
  }, [drawn]);
  if (!drawn) return null;
  const { r, targets } = drawn,
    many = r.top.length > 1,
    keys = resultKeys(r.top);
  return (
    <>
      {r.top.map((t, i) => (
        <LoopCard key={keys[i]} result={t} targetCount={targets.length} rank={many ? i + 1 : 0} top={r.top[0]} />
      ))}
    </>
  );
}
/** Results: 1 or 3, the game's pill switch */
export function ResultsCount() {
  return (
    <>
      <span id="c-top-l">Results</span>
      <RadioPills
        tag="span"
        class="seg"
        id="c-top"
        labelledBy="c-top-l"
        value={fields.decksTop.value}
        onPick={setDecksTop}
        pills={[
          { v: '1', dataV: '1', body: '1' },
          { v: '3', dataV: '3', title: 'Three results: a slower search', body: '3' },
        ]}
      />
    </>
  );
}
// one result: its odds, the three or four umas, the deck(s), the schedule and affinity buttons. rank: 1-based, 0 when
// only one result is shown; top: the #1 result (to say what differs from it)
function LoopCard({
  result,
  targetCount,
  rank,
  top,
}: {
  result: LoopResult;
  targetCount: number;
  rank: number;
  top: LoopResult;
}) {
  const loop = result.loop,
    sameLoop = rank > 1 && loopKey(top.loop) === loopKey(loop);
  return (
    <article class={`loop${rank <= 1 ? ' first' : ''}`}>
      <div class="lhead">
        {rank ? <span class="rank">#{rank}</span> : null}
        <span class="odds">
          <b>{fmtIn(loop.p_all)}</b> runs to land all {targetCount}
        </span>
      </div>
      {sameLoop ? (
        <p class="same">Same loop as #1 (the same umas and outfits); only the deck differs.</p>
      ) : (
        <Members loop={loop} />
      )}
      {result.deck && <Decks loop={loop} shared={result.deck} decks={result.decks || null} />}
      <Actions loop={loop} targetCount={targetCount} />
    </article>
  );
}
// the loop's members: face and name (a button opening her panel), the outfit named, the targets on her own outfit's
// skill list (she has them when she is the trainee) and the odds with her as the trainee. One outfit named, never "any
// outfit" (her popup says when any would do, or which others are as good), as the settings were when the result was
// drawn (untracked: an outfit owned later changes the next result, not this one's names)
function Members({ loop }: { loop: ResultLoop }) {
  const picks = untracked(() => loop.members.map((m, k) => pickOutfit(m.id, loop.outfits[k].outfits)));
  return (
    <div class={`members${loop.members.length === 3 ? ' n3' : ''}`}>
      {loop.members.map((m, k) => (
        <Member key={m.id} loop={loop} k={k} pick={picks[k]} />
      ))}
    </div>
  );
}
function Member({ loop, k, pick }: { loop: ResultLoop; k: number; pick: ReturnType<typeof pickOutfit> }) {
  const m = loop.members[k],
    offered = loop.outfits[k],
    outfitId = offered.outfits.length === 1 ? offered.outfits[0].id : null,
    outfit = pick.list[0]?.label || '',
    face = pick.list[0]?.id ?? null;
  const u = gameData().umas.find((x) => x.id === m.id),
    off = !!u && (umaState(u) === 'out' || (outfitId !== null && fields.outfits.value[outfitId] === false));
  const opener = useOpener();
  return (
    <div
      class={`mem${off ? ' off' : ''}`}
      data-uma={String(m.id)}
      data-outfit={String(outfitId ?? '')}
      data-outfit-label={outfit}
      data-outfit-note={outfitNote(pick)}
    >
      <button type="button" class="head memb" aria-haspopup="dialog" {...opener}>
        <UmaAv key={`${m.id}:${face}`} uma={m.id} outfit={face} size={38} />
        <span class="nm">{m.name}</span>
      </button>
      <span class="of">{outfit}</span>
      <span class="sk">
        {offered.covers.map((cover) => {
          const names = namesOf(cover.group),
            gold = cover.level === 2;
          return (
            <span key={cover.group} class={`spark${gold ? ' g' : ''}`}>
              <SkillIcon group={cover.group} gold={gold} />
              {gold ? names.gd || names.w : names.w}
            </span>
          );
        })}
      </span>
      <span class="own" title="The odds that every target lands in a run with her as the trainee">
        as trainee: <span class="num">{fmtIn(loop.per_trainee[k])}</span>
      </span>
    </div>
  );
}
// a member's portrait (her outfit's when the loop fixes one): a button opening her panel, as her name at the top does,
// with her name as its tooltip
function FaceButton({ loop, k, size }: { loop: ResultLoop; k: number; size: number }) {
  const member = loop.members[k],
    outfits = loop.outfits[k].outfits,
    one = outfits.length === 1 ? outfits[0] : null,
    name = member.name + (one ? ` ${one.label}` : '');
  const opener = useOpener();
  return (
    <button
      type="button"
      class="facebtn"
      data-uma={member.id}
      title={name}
      aria-label={name}
      aria-haspopup="dialog"
      {...opener}
    >
      <UmaAv key={`${member.id}:${one?.id}`} uma={member.id} outfit={one?.id ?? null} size={size} />
    </button>
  );
}
// the deck(s) of a result: one deck for all, or the deck most members share and the swaps from it
function Decks({ loop, shared, decks }: { loop: ResultLoop; shared: ResultDeck; decks: ResultDeck[] | null }) {
  const members = loop.members;
  if (!decks || decks.every((d) => deckKey(d) === deckKey(shared)))
    return (
      <DeckBlock deck={shared} title={decks ? `Deck for all ${members.length === 3 ? 'three' : 'four'}` : 'Deck'} />
    );
  const users = allIndices(members).filter((k) => deckKey(decks[k]) === deckKey(shared));
  // each other deck once, with the members on it
  const others = [...new Set(decks.map(deckKey))].filter((key) => key !== deckKey(shared));
  return (
    <>
      <DeckBlock
        deck={shared}
        title={
          <>
            Deck for{' '}
            <span class="dwho">
              {users.map((k) => (
                <FaceButton key={k} loop={loop} k={k} size={42} />
              ))}
            </span>
            <span class="vh">{users.map((k) => members[k].name).join(', ')}</span>
          </>
        }
      />
      {/* a label where the other decks start: each row below is another uma's deck, by its changes to the deck above */}
      <p class="swhead">Swaps</p>
      {others.map((key) => {
        const who = allIndices(members).filter((j) => deckKey(decks[j]) === key);
        return <SwapRow key={key} loop={loop} shared={shared} deck={decks[who[0]]} who={who} />;
      })}
    </>
  );
}
// one card of a deck as a button opening its panel: its art, name and title, and what to do with it in IT
function CardChip({ deck, i }: { deck: ResultDeck; i: number }) {
  const named = deck.cards[i],
    borrowed = /\(borrowed\)$/.test(named),
    base = named.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, ''),
    pal = (deck.pal_cards || []).includes(base);
  const c = gameData().cardById.get(deck.card_ids[i]) || gameData().cards.find((x) => x.name === base); // plain pals have a name but no entry in card_ids
  // pick: the skill to prioritise in IT: the search's choice at a chain link ("link 1: Uma Stan"), or what a pal's
  // credited event choice needs (derived from the event data: c.pri)
  const endAt = (deck.end_chain_at || [])[i] || 0,
    pick = (deck.choices || [])[i] || (c?.pri ? c.pri.join(', ') : '');
  const pickNote = (pick ? ` · prioritise ${pick}` : '') + (endAt ? ` · ${chainEndNote(c, endAt)}` : '');
  const title = c ? cardTitle(c) : { nm: base, ti: '' };
  const tag = borrowed ? 'borrowed' : pal ? 'pal' : '',
    full = title.nm + (title.ti ? ` · ${title.ti}` : '') + (tag ? ` (${tag})` : '') + pickNote;
  // greyed while it is marked not owned (a borrowed card is not yours to own)
  const off = !!c && !borrowed && fields.cardsOff.value[c.id] === false;
  const opener = useOpener();
  return (
    <button
      type="button"
      class={`dc${borrowed ? ' bor' : pal ? ' pal' : ''}${off ? ' off' : ''}`}
      data-cid={c ? c.id : ''}
      data-name={title.nm}
      data-end={endAt || undefined}
      title={full}
      aria-label={full}
      aria-haspopup="dialog"
      {...opener}
    >
      <CardAv c={c || null} size={96} titled={false} />
      <span class="dn">{full}</span>
      {pick ? <i>prioritise {pick}</i> : null}
      {endAt ? <i>{chainEndNote(c, endAt)}</i> : null}
    </button>
  );
}
const chips = (deck: ResultDeck, indices: number[]) =>
  borrowedLast(deck, indices).map((i: number) => <CardChip key={deck.cards[i]} deck={deck} i={i} />);
// the (i) button by a deck's title, showing its hint odds
function InfoButton({ open, toggle }: { open: boolean; toggle: () => void }) {
  return (
    <button
      type="button"
      class="oinfo"
      aria-expanded={open}
      aria-label="Hint odds from this deck"
      title="Hint odds from this deck"
      onClick={toggle}
    >
      i
    </button>
  );
}
function DeckBlock({ deck, title }: { deck: ResultDeck; title: ComponentChildren }) {
  const [open, setOpen] = useState(false);
  return (
    <div class="deck">
      <div class="dtitle">
        <b>{title}</b>
      </div>
      <InfoButton open={open} toggle={() => setOpen(!open)} />
      <div class="dcards">{chips(deck, allIndices(deck.cards))}</div>
      <HintOdds deck={deck} open={open} />
    </div>
  );
}
// per target the deck can hint: the gold's odds, then the white's (the chance of at least the white): the two numbers
// the landing odds depend on. Shown on demand (the (i) button by the deck's title): one tile per hint: icon, name and
// percentage over a bar (the icons alone don't tell skills apart: the game draws one icon per effect type)
function HintOdds({ deck, open }: { deck: ResultDeck; open: boolean }) {
  const tile = (group: number, gold: boolean, name: string, p: number, tip = '') => (
    <span key={`${group}:${gold}`} class={`dsk${gold ? ' g' : ''}`} title={`${tip || name}: ${percent(p)}`}>
      <SkillIcon group={group} gold={gold} />
      <span class="dnm">{name}</span>
      <span class="dbar" aria-hidden="true">
        <span style={{ width: percent(p) }} />
      </span>
      <b>{percent(p)}</b>
    </span>
  );
  const tiles = deck.odds
    .filter((o) => o.white > SHOWN || o.gold > SHOWN)
    .flatMap((o) => {
      const names = namesOf(o.group),
        white = atLeastWhite(o);
      return [
        o.gold > SHOWN && tile(o.group, true, names.gd || names.w, o.gold),
        white - o.gold > SHOWN && tile(o.group, false, names.w, white, o.gold > SHOWN ? `${names.w} or its gold` : ''),
      ];
    });
  return (
    <div class="dodds" hidden={!open}>
      <span class="dlab">Hint odds from this deck</span>
      {tiles.some(Boolean) ? tiles : <span>This deck gives none of the targets.</span>}
    </div>
  );
}
// what a swap changes: the hint odds that move against the shared deck (the gold's, then at least the white's),
// largest change first; two shown, a click on the list shows them all (and again folds it). who: the members on
// that deck; a hint all of them have on their own outfit's skill list is no change for them (its gold: they have the
// gold; the white: they have at least the white), so it isn't listed (her deck drops it because she doesn't need it)
function SwapDeltas({
  loop,
  shared,
  deck,
  who,
}: {
  loop: ResultLoop;
  shared: ResultDeck;
  deck: ResultDeck;
  who: number[];
}) {
  const [more, setMore] = useState(false);
  const before = new Map(shared.odds.map((o) => [o.group, o])),
    changes: { group: number; gold: boolean; name: string; from: number; to: number }[] = [];
  const ownLevel = (group: number) =>
    Math.min(...who.map((k) => (loop.outfits[k].covers.find((c) => c.group === group) || { level: 0 }).level));
  for (const o of deck.odds) {
    const b = before.get(o.group),
      names = namesOf(o.group),
      level = ownLevel(o.group);
    if (level < 2 && Math.abs(o.gold - (b ? b.gold : 0)) >= SHOWN)
      changes.push({ group: o.group, gold: true, name: names.gd || names.w, from: b ? b.gold : 0, to: o.gold });
    if (
      level < 1 &&
      Math.abs(atLeastWhite(o) - atLeastWhite(b)) >= SHOWN &&
      (atLeastWhite(o) - o.gold > SHOWN || atLeastWhite(b) - (b ? b.gold : 0) > SHOWN)
    )
      changes.push({ group: o.group, gold: false, name: names.w, from: atLeastWhite(b), to: atLeastWhite(o) });
  }
  changes.sort((x, y) => Math.abs(y.to - y.from) - Math.abs(x.to - x.from));
  if (!changes.length) return null;
  const one = (x: (typeof changes)[number], extra: boolean) => (
    <span key={`${x.group}:${x.gold}`} class={`swd1${extra ? ' extra' : ''}${x.to < x.from ? ' down' : ''}`}>
      <SkillIcon group={x.group} gold={x.gold} />
      {x.name} <b>{`${percent(x.from)} → ${percent(x.to)}`}</b>
    </span>
  );
  if (changes.length <= 2) return <span class="swd">{changes.map((x) => one(x, false))}</span>;
  const hidden = changes.length - 2;
  return (
    <button type="button" class="swd more" aria-expanded={more} onClick={() => setMore(!more)}>
      {changes.map((x, i) => one(x, i >= 2))}
      <span class="swmore" data-n={hidden}>
        {more ? 'Show less' : `+${hidden} more`}
      </span>
    </button>
  );
}
// another member's deck by its changes to the shared one, one row: her face (each with her name as its tooltip), the
// cards that go (greyed) ➜ the cards that come in, and what it does to the odds. `who`: the members on this deck
function SwapRow({
  loop,
  shared,
  deck,
  who,
}: {
  loop: ResultLoop;
  shared: ResultDeck;
  deck: ResultDeck;
  who: number[];
}) {
  const [open, setOpen] = useState(false);
  const names = who.map((k) => loop.members[k].name);
  const whose = `${names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`}’s deck`; // "X’s deck", "X and Y’s deck"
  const out = allIndices(shared.cards).filter((i) => !deck.cards.includes(shared.cards[i]));
  const comingIn = allIndices(deck.cards).filter((i) => !shared.cards.includes(deck.cards[i]));
  return (
    <div class="deck swap">
      <div class="swrow">
        <span class="swho">
          {who.map((k) => (
            <FaceButton key={k} loop={loop} k={k} size={64} />
          ))}
          <span class="vh">{whose}:</span>
        </span>
        <div class="swcards">
          <span class="swc swout">
            <span class="vh">out:</span>
            {chips(shared, out)}
          </span>
          <span class="swto">
            <span class="swa" aria-hidden="true">
              <Arrow />
            </span>
            <span class="swc swin">
              <span class="vh">in:</span>
              {chips(deck, comingIn)}
            </span>
          </span>
        </div>
        <SwapDeltas loop={loop} shared={shared} deck={deck} who={who} />
      </div>
      <InfoButton open={open} toggle={() => setOpen(!open)} />
      <HintOdds deck={deck} open={open} />
    </div>
  );
}
// the game's green arrow (white-edged, dark green outline) between the card that goes and the one that comes in
const Arrow = () => (
  // biome-ignore lint/a11y/noSvgWithoutTitle: decoration, in an aria-hidden span (the "out:" and "in:" labels say it)
  <svg viewBox="0 0 30 24" width="30" height="24" focusable="false">
    <path
      d="M2 8h13V2.5L28 12 15 21.5V16H2z"
      fill="#7ecf08"
      stroke="#3c7a00"
      stroke-width="2"
      stroke-linejoin="round"
    />
    <path
      d="M4.5 10h12.5V7.3L24 12"
      fill="none"
      stroke="#fff"
      stroke-opacity=".55"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
  </svg>
);
// the result's dialogs: the odds of each count landed (the solver's, exact over the landed sets, as the career
// planner's), the race schedule and the affinity
function Actions({ loop, targetCount }: { loop: ResultLoop; targetCount: number }) {
  const landed = loop.landed;
  return (
    <div class="row lacts">
      {landed && (
        <button
          type="button"
          class="btn sbtn"
          aria-haspopup="dialog"
          onClick={() =>
            openOdds({
              title: 'Odds of this loop',
              n: targetCount,
              ex: landed,
              pAll: loop.p_all,
              expected: landed.reduce((sum, p, k) => sum + k * p, 0),
            })
          }
        >
          Odds
        </button>
      )}
      <button type="button" class="btn sbtn" onClick={() => openSched(loop)}>
        Race schedule
      </button>
      {loop.detail && (
        <button type="button" class="btn sbtn" onClick={() => openWhy(loop)}>
          Affinity
        </button>
      )}
    </div>
  );
}
