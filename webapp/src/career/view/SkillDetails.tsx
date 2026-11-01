// The game-style Skill Details popup: a target skill of the career on show (the tree's targets, Try a run's skills), with
// her chance of it, where it comes from, which legacies carry it and which cards give it. One dialog, there from load
// (empty); `openSkill` fills and opens it, and the browser gives the focus back to the tile that opened it.
import { computed, signal } from '@preact/signals';
import type { Card } from '../../data.ts';
import type { CareerBuy } from '../job.ts';
import { umaName } from '../../veterans/names.ts';
import { CardAv, SkillIcon, UmaAv } from '../../ui/Images.tsx';
import { Dialog, DialogBody, DialogClose, DialogFoot, DialogTitle } from '../../ui/Dialog.tsx';
import { Guard } from './Guard.tsx';
import { popupHeading } from './parts.ts';
import { skillTile } from '../../ui/skillTile.ts';
import { cx } from 'tailwind-variants/lite';

/** the skill on show: `b` its buying report; `got` 'gold' | 'white' | null (not acquired), or undefined for a career
 *  not run yet (the popup then gives her chance to land it); `group` and `goldIcon` its icon; `gold` (a career not run)
 *  its gold is the likelier; `legacies` her six legacies, each lit where one carries it; `cards` the deck cards that
 *  give it, each with how ("hint", "gold hint", "event at link 3") */
export interface SkillSpec {
  b: Readonly<CareerBuy>;
  name: string;
  got?: 'gold' | 'white' | null;
  group: number;
  goldIcon: boolean;
  gold?: boolean;
  legacies?: readonly { chara: number; outfit?: number | null | undefined; carries: boolean }[];
  cards?: readonly { card: Card; how: string }[];
}
const shown = signal<{ spec: SkillSpec | null; open: boolean }>({ spec: null, open: false });
// its guard draws again for a new skill only (a close is no reason to try failing content again)
const shownSpec = computed(() => shown.value.spec);
/** the popup for one skill */
export function openSkill(spec: SkillSpec) {
  shown.value = { spec, open: true };
}
const close = () => {
  shown.value = { ...shown.peek(), open: false };
};
export function SkillDetails() {
  const { spec, open } = shown.value;
  return (
    <Dialog size="md" id="sk-dlg" kind="popup" labelledBy="sk-title" open={open} onClose={close}>
      <DialogTitle id="sk-title">Skill Details</DialogTitle>
      <Guard what="Skill Details" also={shownSpec}>
        {spec && <Skill spec={spec} />}
      </Guard>
      <DialogFoot>
        <DialogClose onClick={close} />
      </DialogFoot>
    </Dialog>
  );
}
// her legacies with their places (fixed: an uma can stand in two, so the place keys each portrait)
const places = <T,>(list: readonly T[]) => list.map((x, place) => [place, x] as const);
// a line of a list (a source, a card, a total): its label and its figure at the ends, on the game's white cell
const LINE = 'bg-gm-cell border-gm-line flex justify-between gap-2.5 rounded-md border px-2.5 py-1.25 font-bold';
const pctS = (p: number) => (p >= 0.995 ? 'always' : p < 0.005 ? '<1%' : `${Math.round(p * 100)}%`);
function Skill({
  spec: { b, name, got, group, goldIcon, gold = false, legacies = [], cards = [] },
}: {
  spec: SkillSpec;
}) {
  const ahead = got === undefined; // (a career not run yet: chances only)
  // where it comes from, each with its chance (a source under half a percent isn't listed)
  const sources: { label: string; chance: number }[] = [];
  if (b.kind === 'scenario') sources.push({ label: 'The scenario', chance: 1 });
  if (b.outfit === 2) sources.push({ label: 'Her own skill (gold)', chance: 1 });
  else if (b.outfit === 1) sources.push({ label: 'Her own skill', chance: 1 });
  // the deck's gold, else its white (`deck_white` is given no gold): the two add up to its chance of a hint, `deckHint`
  const deckGold = b.deck_gold || 0,
    deckWhite = (1 - deckGold) * (b.deck_white || 0);
  if (deckGold > 0.005) sources.push({ label: 'Deck (its gold)', chance: deckGold });
  if (deckWhite > 0.005) sources.push({ label: 'Deck', chance: deckWhite });
  if ((b.own_event || 0) > 0.005) sources.push({ label: 'Her story events', chance: b.own_event || 0 });
  if ((b.inherit || 0) > 0.005) sources.push({ label: 'Legacies (inheritance)', chance: b.inherit || 0 });
  const totals = [
    { label: 'Acquired', chance: b.gold + b.white },
    { label: 'Lands as a spark', chance: b.land_gold + b.land_white },
  ];
  const off = !(got || ahead);
  return (
    <DialogBody>
      <div class={skillTile({ tone: got === 'gold' || (ahead && gold) ? 'gold' : off ? 'off' : 'white' })}>
        <SkillIcon group={group} gold={goldIcon} dim={off} cls="size-7.5" />
        <span class="text-body leading-tight wrap-anywhere">{name}</span>
      </div>
      <p class={cx('font-bold', off ? 'text-gm-ink2' : 'text-gm-ink')}>
        {ahead
          ? `Lands as a spark: ${pctS(b.land_gold + b.land_white)}${gold ? ' (through its gold)' : ''}`
          : got
            ? `Acquired${got === 'gold' ? ' (gold)' : ''} in this result`
            : 'Not acquired in this result'}
      </p>
      <div class={popupHeading}>Where it comes from</div>
      <ul class="grid gap-1">
        {sources.map(({ label, chance }) => (
          <li key={label} class={LINE}>
            <span>{label}</span>
            <b class="tabular-nums">{pctS(chance)}</b>
          </li>
        ))}
      </ul>
      {legacies.length > 0 && (
        <>
          <div class={popupHeading}>Legacies carrying it</div>
          {/* (data-faces: the checks count them) */}
          <div class="flex flex-wrap gap-1" data-faces="">
            {places(legacies).map(([place, l]) => (
              <UmaAv
                key={`${place}:${l.chara}:${l.outfit}`}
                uma={l.chara || 0}
                outfit={l.outfit}
                size={32}
                dim={l.carries ? undefined : 'faint'}
                title={`${umaName(l.chara)}${l.carries ? ': carries it' : ': no spark of it'}`}
              />
            ))}
          </div>
        </>
      )}
      {cards.length > 0 && (
        <>
          <div class={popupHeading}>Cards that give it</div>
          <ul class="grid gap-1">
            {cards.map(({ card, how }) => (
              <li key={card.id} class={LINE}>
                <span class="inline-flex min-w-0 items-center gap-2">
                  <CardAv c={card} size={28} />
                  <span>{card.name.replace(/^\[[^\]]*\] /, '')}</span>
                </span>
                <b class="text-note whitespace-nowrap tabular-nums">{how}</b>
              </li>
            ))}
          </ul>
        </>
      )}
      <div class="grid grid-cols-2 gap-1">
        {totals.map(({ label, chance }) => (
          <div key={label} class={LINE}>
            <span>{label}</span>
            <b class="tabular-nums">{pctS(chance)}</b>
          </div>
        ))}
      </div>
    </DialogBody>
  );
}
