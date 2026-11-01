// ---------- the deck card panel (popover.ts holds which one is open): a card's name, gifts, chain-end advice and
// Owned switch, or a loop member's name, outfit and Owned switch; each with its GameTora link ----------
// One slot in the results column (each entry renders it into #pop-root) and one in every ui/Dialog: the slot where the
// panel's opener sits draws it. Placed against its opener and focused from a layout effect, once per opening.
import { useLayoutEffect, useRef } from 'preact/hooks';
import { gameData } from '../data.ts';
import type { Card, Uma } from '../data.ts';
import { settings, setOwned } from '../settings.ts';
import { goldGiven } from '../job.ts';
import { umaState } from '../roster.ts';
import { cardTitle, cardGives, chainEndAt } from '../cards.ts';
import type { CardGift } from '../cards.ts';
import { must } from '../must.ts';
import { popBack, popOpen, setPanel } from '../popover.ts';
import type { PopOpen, PopSubject } from '../popover.ts';

const GAP = 8; // px between the panel and the element it belongs to (beside it); below it, 6
const focusables = (pop: HTMLElement) => [...pop.querySelectorAll<HTMLElement>('input, a[href], button')];
// beside: a deck card's panel opens to its right (else its left) at its top, so it never covers the hint odds under the
// deck; without room on either side (phones), below it like the others. Positions are in the host's scrolled box. In a
// popup: placed against the screen, under the card, so the popup's edges don't clip it.
function place(pop: HTMLElement, { opener: anchor, beside }: PopOpen, inPopup: boolean) {
  if (inPopup) {
    const rect = anchor.getBoundingClientRect(),
      width = pop.offsetWidth;
    pop.style.left = `${Math.max(8, Math.min(rect.left, innerWidth - width - 8))}px`;
    pop.style.top = `${Math.min(rect.bottom + 6, innerHeight - pop.offsetHeight - 8)}px`;
    return;
  }
  const host = pop.offsetParent || document.body,
    hostRect = host.getBoundingClientRect(),
    rect = anchor.getBoundingClientRect(),
    width = pop.offsetWidth;
  const right = rect.right - hostRect.left + GAP,
    left = rect.left - hostRect.left - GAP - width;
  if (beside && right + width <= host.clientWidth) {
    pop.style.left = `${host.scrollLeft + right}px`;
    pop.style.top = `${host.scrollTop + rect.top - hostRect.top}px`;
  } else if (beside && left >= 0) {
    pop.style.left = `${host.scrollLeft + left}px`;
    pop.style.top = `${host.scrollTop + rect.top - hostRect.top}px`;
  } else {
    pop.style.left = `${host.scrollLeft + Math.max(0, Math.min(rect.left - hostRect.left, host.clientWidth - width))}px`;
    pop.style.top = `${host.scrollTop + rect.bottom - hostRect.top + 6}px`;
  }
}
/** the panel's slot: `home`, the id of the ui/Dialog it sits in (null: the results column, where the hidden panel is
 *  always drawn; a popup's slot draws nothing until a panel opens in it) */
export function CardPop({ home }: { home: string | null }) {
  const now = popOpen.value,
    here = now && now.home === home ? now : null,
    ref = useRef<HTMLDivElement>(null);
  // each opening: placed against its opener, the focus in its first control (or itself)
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per opening (`here` is a module signal's record, which Biome reads as constant)
  useLayoutEffect(() => {
    const pop = ref.current;
    if (!here || !pop) return;
    setPanel(pop);
    place(pop, here, home !== null);
    (focusables(pop)[0] || pop).focus({ preventScroll: true });
  }, [here]);
  if (!here && home !== null) return null;
  const subject = here?.subject;
  return (
    <div
      ref={ref}
      class={`cpop${home !== null ? ' inpopup' : ''}`}
      hidden={!here}
      role="dialog"
      tabIndex={-1}
      aria-label={subject ? titleOf(subject) : undefined}
      onKeyDown={(e) => {
        if (e.key !== 'Tab') return;
        const items = focusables(e.currentTarget),
          i = items.indexOf(document.activeElement as HTMLElement);
        if (!items.length || (e.shiftKey ? i <= 0 : i === items.length - 1)) {
          e.preventDefault();
          popBack();
        }
      }}
    >
      {subject?.kind === 'card' && <CardBody subject={subject} />}
      {subject?.kind === 'uma' && <UmaBody subject={subject} />}
    </div>
  );
}
const umaOf = (id: number) =>
  must(
    gameData().umas.find((x) => x.id === id),
    "a loop's member is an uma of the data",
  );
const cardOf = (cid: number | null) => (cid === null ? undefined : gameData().cardById.get(cid));
// the panel's name: hers (with the outfit the loop fixes), or the card's
function titleOf(subject: PopSubject) {
  if (subject.kind === 'uma') return umaOf(subject.uma).name + (subject.outfit ? ` ${subject.outfitLabel}` : '');
  const c = cardOf(subject.cid);
  return c ? cardTitle(c).nm : subject.name;
}
// a member's panel: one "Owned" switch. When the loop uses a specific outfit, that outfit is what is owned or not
// (another outfit would be a different trainee); with "any outfit" it is the uma.
function UmaBody({ subject: { uma, outfit, outfitLabel, outfitNote } }: { subject: PopSubject & { kind: 'uma' } }) {
  const u = umaOf(uma),
    owned = outfit ? umaState(u) !== 'out' && settings.value.outfits[outfit] !== false : umaState(u) !== 'out';
  const toggle = (on: boolean) => {
    const roster = { ...settings.value.roster },
      outfits = { ...settings.value.outfits };
    if (on) {
      if (roster[u.id] === 'out') delete roster[u.id];
      if (outfit) delete outfits[outfit];
      else for (const o of u.outfits || []) delete outfits[o.id];
    } else if (outfit) outfits[outfit] = false;
    else roster[u.id] = 'out';
    setOwned({ roster, outfits });
  };
  return (
    <>
      <div>
        <b>{u.name}</b>
        <div class="ctt">{outfitLabel}</div>
        {outfitNote && <div class="onote">{outfitNote}</div>}
      </div>
      <div class="pfoot">
        <label class="sw">
          <input
            type="checkbox"
            // biome-ignore lint/a11y/useAriaPropsForRole: a native checkbox's checked state is the switch's (ARIA in HTML: no aria-checked on a checkbox)
            role="switch"
            data-act="uma"
            checked={owned}
            onChange={(e) => toggle(e.currentTarget.checked)}
          />
          Owned
        </label>
        <a class="gtl" href={umaUrl(u, outfit)} target="_blank" rel="noopener">
          GameTora ↗︎
        </a>
      </div>
    </>
  );
}
function CardBody({ subject: { cid, name, borrowed, end } }: { subject: PopSubject & { kind: 'card' } }) {
  const c = cardOf(cid),
    title = c ? cardTitle(c) : { nm: name, ti: '' },
    owned = !!c && settings.value.cardsOff[c.id] !== false;
  const groups = new Set(
      settings.value.targets.filter((x) => !goldGiven(settings.value.scen, x.group)).map((x) => x.group),
    ),
    gifts = c ? cardGives(c, groups) : [];
  return (
    <>
      <div>
        <b>{title.nm}</b>
        {title.ti && <div class="ctt">{title.ti}</div>}
      </div>
      {borrowed && <div>Borrowed.</div>}
      {gifts.length > 0 && (
        <div class="pgives">
          <Gifts gifts={gifts} />
        </div>
      )}
      {end > 0 && <ChainEnd c={c} link={end} />}
      <div class="pfoot">
        {c && !borrowed ? (
          <label class="sw">
            <input
              type="checkbox"
              // biome-ignore lint/a11y/useAriaPropsForRole: a native checkbox's checked state is the switch's (ARIA in HTML: no aria-checked on a checkbox)
              role="switch"
              data-act="own"
              checked={owned}
              onChange={(e) => {
                const { [c.id]: _, ...rest } = settings.value.cardsOff;
                setOwned({ cardsOff: e.currentTarget.checked ? rest : { ...rest, [c.id]: false } });
              }}
            />
            Owned
          </label>
        ) : (
          <span />
        )}
        {c && (
          <a class="gtl" href={gametoraUrl(c)} target="_blank" rel="noopener">
            GameTora ↗︎
          </a>
        )}
      </div>
    </>
  );
}
/** the gifts as the game's small labels */
function Gifts({ gifts }: { gifts: CardGift[] }) {
  return (
    <>
      {gifts.map((x) => (
        <span key={`${x.skill}:${x.note}:${x.tip}`} class={`gv${x.gold ? ' g' : ''}`} title={x.tip}>
          {x.skill} <small>{x.note}</small>
        </span>
      ))}
    </>
  );
}
// what to prioritise for IT to take the chain-ending choice the search chose, and when it may not. In the run data IT
// took a prioritised skill's choice in every one of 312 readings where only that choice held one, across card events
// (269 of them Sasami Anshinzawa SSR's finale); the rule is carried over to chain-ending choices here. It took it less
// often when the top choice held a prioritised skill too (41 of 56) or the skill's hint was already Lv 5 from another
// source (16 of 28): small samples, so "may not" (docs/RESEARCH.md section 9)
function ChainEnd({ c, link }: { c: Card | undefined; link: number }) {
  const { take, others } = chainEndAt(c, link);
  if (!take) return <div>{`Take the chain-ending choice at link ${link}.`}</div>;
  const targets = new Set(settings.value.targets.map((x) => x.group)),
    rivals = others.filter((x) => targets.has(x.group)).map((x) => x.name),
    otherNames = others.map((x) => x.name).join(' or ');
  let caveat: string;
  if (rivals.length)
    caveat = `${rivals.join(' and ')} ${rivals.length > 1 ? 'are' : 'is'} also a target: prioritising both makes the end uncertain.`;
  else if (otherNames)
    caveat = `IT may not take it if ${otherNames} is also prioritised, or if ${take}'s hint is already Lv 5.`;
  else caveat = `IT may not take it if ${take}'s hint is already Lv 5.`;
  return (
    <>
      <div>{`Prioritise ${take} in IT: it takes the chain-ending choice at link ${link}.`}</div>
      <div class="onote">{caveat}</div>
    </>
  );
}
// a name as a GameTora URL slug: lowercase, '.' and "'" dropped, every other run of punctuation a hyphen
const slugOf = (name: string) =>
  name
    .toLowerCase()
    .replace(/[.']/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
// GameTora's character page: /characters/<outfit id>-<slug>, the slug from her support cards' url_name
// (export_page.py), else made from her name; with "any outfit" her original outfit (<chara id>01)
function umaUrl(u: Uma, outfitId: number | null) {
  return `https://gametora.com/umamusume/characters/${outfitId || u.id * 100 + 1}-${u.gt || slugOf(u.name)}`;
}
// GameTora's card page, by the url_name in its own card list (export_page.py; group cards have names a slug can't
// guess); for a card missing from that list, id + character name, lowercase, punctuation dropped.
function gametoraUrl(c: Card) {
  const page = c.gt || `${c.id}-${slugOf(c.name.replace(/^\[.*?\]\s*/, ''))}`; // the name without its [title]
  return `https://gametora.com/umamusume/supports/${page}`;
}
