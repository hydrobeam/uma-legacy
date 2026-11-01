// ---------- the Support cards panel: All / None owned, Import, the count; the filter and the order; the card picker ----------
// The picker lists the cards that give a target the deck must supply (ones the scenario guarantees left out) and the
// pals, by rarity, in the chosen order; it is drawn again only when that list changes (targets, scenario, order). A tile
// is a toggle, pressed = owned; its name is its visible text (name, title, what it gives), so voice control matches it.
// Its pressed state and the filter's verdict are each bound to a computed, so a click, All / None or a keystroke
// redraws no tile. Drawn in the Support cards dialog (ui/Setup.tsx).
import { computed, signal, useComputed } from '@preact/signals';
import { Fragment } from 'preact';
import { useRef } from 'preact/hooks';
import { SCENARIOS, gameData } from '../data.ts';
import type { Card } from '../data.ts';
import { fields, settings } from '../settings.ts';
import { goldGiven } from '../job.ts';
import { cardGives, cardTitle, ownAllCards, ownNoCards, toggleCard } from '../cards.ts';
import type { CardGift } from '../cards.ts';
import { CardAv } from './Images.tsx';
import { OwnedImport, useSaid } from './OwnedImport.tsx';
import { SettingsSort } from './SortMenu.tsx';
import { useMoreFade } from './moreFade.ts';
import { Button } from './Button.tsx';

// the filter's text, trimmed and lower-cased: a card shows when its name holds it
const query = signal('');
const matches = (c: Card, q: string) => !q || c.name.toLowerCase().includes(q);
interface Listed {
  c: Card;
  gifts: CardGift[];
  /** what the tile says when the chosen scenario does not allow it (Team Sirius in Unity Cup and Grand Concert: the
   *  search leaves it out), else null */
  banNote: string | null;
}
const scenarioName = (v: string) => SCENARIOS.find((s) => s.v === v)?.name;
// the picker's sections: each rarity's cards that give a target the deck still has to give, and its pals (R pals stay
// usable without R cards), in the chosen order
const sections = computed(() => {
  const scen = fields.scen.value || 'unity',
    groups = new Set(fields.targets.value.filter((t) => !goldGiven(scen, t.group)).map((t) => t.group));
  // R cards (not pals): no setting turns them on (settings.ts DEFAULTS), so this is read without subscribing
  const rCards = settings.peek().rcards === true;
  const dir = fields.cdir.value === 'desc' ? -1 : 1,
    byName = fields.csort.value === 'name',
    sortName = (c: Card) => {
      const t = cardTitle(c);
      return `${t.nm} ${t.ti}`;
    };
  const order = (a: Listed, b: Listed) =>
    dir * (byName ? sortName(a.c).localeCompare(sortName(b.c)) : a.c.rel - b.c.rel || a.c.id - b.c.id);
  const rarities: [string, (c: Card) => boolean][] = [
    ['SSR', (c) => c.rar === 3],
    ['SR', (c) => c.rar === 2],
    ['R', (c) => c.rar === 1 && (c.pal || rCards)],
  ];
  return rarities
    .map(([label, inSection]) => ({
      label,
      list: gameData()
        .cards.filter(inSection)
        .map((c) => ({
          c,
          gifts: cardGives(c, groups),
          banNote: (c.ban || []).includes(scen) ? `Not allowed in ${scenarioName(scen)}` : null,
        }))
        .filter((x) => x.c.pal || x.gifts.length)
        .sort(order),
    }))
    .filter((section) => section.list.length);
});
// the sections with a card the filter shows
const sectionsShown = computed(
  () => new Set(sections.value.filter((s) => s.list.some((x) => matches(x.c, query.value))).map((s) => s.label)),
);
// the cards listed (matching the filter) and how many of them are owned (beside All / None owned)
const countText = computed(() => {
  const off = fields.cardsOff.value,
    listed = sections.value.flatMap((s) => s.list).filter((x) => matches(x.c, query.value));
  return `${listed.filter((x) => off[x.c.id] !== false).length} of ${listed.length} owned`;
});

// one way the card gives a target, as the game's small label
function Gift({ gift }: { gift: CardGift }) {
  return (
    <span class={gift.gold ? 'gv g' : 'gv'} title={gift.tip}>
      {`${gift.skill} `}
      <small>{gift.note}</small>
    </span>
  );
}
function CardTile({ c, gifts, banNote }: Listed) {
  const t = cardTitle(c, false);
  // (the computeds read props: right because each tile is keyed by its card, so `c` never changes)
  const pressed = useComputed(() => (fields.cardsOff.value[c.id] !== false ? 'true' : 'false')),
    hidden = useComputed(() => !matches(c, query.value));
  return (
    <button
      class={banNote ? 'ct ban' : 'ct'}
      type="button"
      aria-pressed={pressed}
      hidden={hidden}
      onClick={() => toggleCard(c.id)}
    >
      <CardAv c={c} size={112} />
      <span class="cn">{t.nm}</span>
      {t.ti ? <span class="cti">{t.ti}</span> : null}
      <span class="cg">
        {gifts.map((gift) => (
          <Gift key={`${gift.note} ${gift.skill}`} gift={gift} />
        ))}
      </span>
      {banNote ? <span class="cban">{banNote}</span> : null}
    </button>
  );
}
function CardSection({ label, list }: { label: string; list: Listed[] }) {
  // (keyed by its label, so the computed's `label` never changes)
  const hidden = useComputed(() => !sectionsShown.value.has(label));
  return (
    <div class="cgroup" hidden={hidden}>
      <h3>{label}</h3>
      <div class="ctiles">
        {list.map((x) => (
          <CardTile key={x.c.id} c={x.c} gifts={x.gifts} banNote={x.banNote} />
        ))}
      </div>
    </div>
  );
}
function CardList({ fadeMore }: { fadeMore: boolean }) {
  const box = useRef<HTMLDivElement>(null);
  useMoreFade(box, fadeMore);
  return (
    <div class="cards" id="cards" ref={box}>
      {sections.value.map((s) => (
        <CardSection key={s.label} label={s.label} list={s.list} />
      ))}
    </div>
  );
}
export function CardsPanel({ fadeMore }: { fadeMore: boolean }) {
  const [said, say] = useSaid();
  return (
    <>
      <div class="row between">
        <h2 id="h-deck">Support deck</h2>
        <div class="row">
          <Button id="c-all" onClick={ownAllCards}>
            All owned
          </Button>
          <Button id="c-none" onClick={ownNoCards}>
            None owned
          </Button>
          <OwnedImport id="c-file" onSaid={say} />
          <span class="hint cnt" id="c-count" role="status">
            {countText}
          </span>
        </div>
      </div>
      <p class="hint">
        {'Only cards that provide relevant skills are shown. '}
        <span id="c-msg" role="status">
          <Fragment key={said.n}>{said.text}</Fragment>
        </span>
      </p>
      <div class="cwrap">
        <div class="row rtools">
          <input
            type="search"
            id="c-filter"
            aria-label="Filter cards"
            placeholder="Filter cards"
            onInput={(e) => {
              query.value = e.currentTarget.value.trim().toLowerCase();
            }}
          />
          <SettingsSort prefix="c" id="cs" what="sort cards" />
        </div>
        <CardList fadeMore={fadeMore} />
      </div>
    </>
  );
}
