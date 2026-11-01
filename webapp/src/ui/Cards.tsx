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
import { OWNED_PANEL, OwnedImport, useSaid } from './OwnedImport.tsx';
import { SettingsSort } from './SortMenu.tsx';
import { useMoreFade } from './moreFade.ts';
import { Button } from './Button.tsx';
import { field } from './Field.ts';
import { cx } from 'tailwind-variants/lite';

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

// one way the card gives a target, as the game's small label: white, a gold in gold; on a card not owned (its tile
// unpressed) on the page's ground in a dashed line. data-gift marks it for the checks.
function Gift({ gift }: { gift: CardGift }) {
  return (
    <span
      class={cx(
        'text-caption group-not-on:bg-bg group-not-on:text-muted group-not-on:border-line group-not-on:border-dashed border px-1 py-px leading-tight font-semibold',
        gift.gold ? 'bg-gold-soft text-gold border-gold' : 'border-hint-edge bg-panel text-ink',
      )}
      data-gift=""
      title={gift.tip}
    >
      {`${gift.skill} `}
      <small class="block font-medium">{gift.note}</small>
    </span>
  );
}
// A card's tile, a toggle (pressed: owned): its art the tile's width, its name, title and gifts under it; not owned, its
// art greyed and its words muted; a card the scenario bans greyed whatever its mark, its note in red. data-ban marks
// the note for the checks.
function CardTile({ c, gifts, banNote }: Listed) {
  const t = cardTitle(c, false);
  // (the computeds read props: right because each tile is keyed by its card, so `c` never changes)
  const pressed = useComputed(() => (fields.cardsOff.value[c.id] !== false ? 'true' : 'false')),
    hidden = useComputed(() => !matches(c, query.value));
  return (
    <button
      class="group grid w-full min-w-0 grid-cols-[minmax(0,1fr)] gap-0.75 rounded-md p-0 text-left focus-visible:outline-offset-3"
      type="button"
      aria-pressed={pressed}
      hidden={hidden}
      onClick={() => toggleCard(c.id)}
    >
      <CardAv c={c} size={112} shape="fill" dim={banNote ? 'always' : 'unpressed'} />
      <span class="text-note group-not-on:text-muted leading-tight font-bold">{t.nm}</span>
      {t.ti ? <span class="text-caption text-muted leading-tight">{t.ti}</span> : null}
      <span class="mt-0.5 grid gap-1">
        {gifts.map((gift) => (
          <Gift key={`${gift.note} ${gift.skill}`} gift={gift} />
        ))}
      </span>
      {banNote ? (
        <span class="text-caption text-out leading-tight font-semibold" data-ban="">
          {banNote}
        </span>
      ) : null}
    </button>
  );
}
// a rarity's section: its name, its tiles eight to a row (as many 96 px ones as fit under 961 px, 80 px ones on a phone)
function CardSection({ label, list }: { label: string; list: Listed[] }) {
  // (keyed by its label, so the computed's `label` never changes)
  const hidden = useComputed(() => !sectionsShown.value.has(label));
  return (
    // biome-ignore lint/a11y/useSemanticElements: a fieldset would bring its min-content width into the list's grid; the section is named by its heading
    <div role="group" aria-labelledby={`cards-${label}`} hidden={hidden}>
      <h3 class="text-caption text-muted mb-1.5 font-bold tracking-widest uppercase" id={`cards-${label}`}>
        {label}
      </h3>
      <div class="grid grid-cols-8 items-start gap-x-2 gap-y-2.5 max-xl:grid-cols-[repeat(auto-fill,minmax(96px,1fr))] max-sm:grid-cols-[repeat(auto-fill,minmax(80px,1fr))]">
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
    <div
      class="data-more:fade-bottom mt-3 grid min-h-0 flex-auto [scrollbar-gutter:stable] gap-3.5 overflow-auto py-1.5 pr-3 pl-1.5"
      id="cards"
      ref={box}
    >
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
      <div class={OWNED_PANEL.top}>
        {/* (the dialog's head shows the title: this one only names the panel) */}
        <h2 class="hidden" id="h-deck">
          Support deck
        </h2>
        <div class={OWNED_PANEL.tools}>
          <Button id="c-all" onClick={ownAllCards}>
            All owned
          </Button>
          <Button id="c-none" onClick={ownNoCards}>
            None owned
          </Button>
          <OwnedImport id="c-file" onSaid={say} />
          <span class={OWNED_PANEL.count} id="c-count" role="status">
            {countText}
          </span>
        </div>
      </div>
      <p class={OWNED_PANEL.hint}>
        {'Only cards that provide relevant skills are shown. '}
        <span id="c-msg" role="status">
          <Fragment key={said.n}>{said.text}</Fragment>
        </span>
      </p>
      <div class="flex min-h-0 flex-auto flex-col">
        <div class={OWNED_PANEL.filters}>
          <input
            type="search"
            id="c-filter"
            class={cx(field.tap, 'flex-1')}
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
