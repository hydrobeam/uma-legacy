// ---------- a row of the game's pill switches as one radio group (P7: one keyboard behaviour for every such row) ----------
// The checked pill is the group's one Tab stop; an arrow key picks the next pill (Right, Down) or the one before (Left,
// Up), wrapping, and focuses it. Each pill is keyed by its value, so the focused one is the same element after the pick.
import type { ComponentChildren } from 'preact';

export interface Pill<V> {
  v: V;
  body: ComponentChildren;
  /** its accessible name, when the body's text is not */
  label?: string;
  title?: string;
  /** a `data-v` attribute (a check or a style names the pill by it) */
  dataV?: string;
}
interface RadioPillsProps<V> {
  /** the group's element ('div' or 'span': the two places it sits in differ) */
  tag?: 'div' | 'span';
  class: string;
  id?: string;
  /** the group's name: an element's id, or the name itself */
  labelledBy?: string;
  label?: string;
  hidden?: boolean;
  /** shown before the pills, inside the group (a visible label) */
  lead?: ComponentChildren;
  pills: Pill<V>[];
  value: V;
  onPick: (v: V) => void;
}
const BACK = ['ArrowLeft', 'ArrowUp'],
  ON = ['ArrowRight', 'ArrowDown'];
export function RadioPills<V extends string | number>(props: RadioPillsProps<V>) {
  const { tag: Tag = 'div', pills, value, onPick } = props;
  const step = (e: KeyboardEvent & { currentTarget: HTMLElement }) => {
    const back = BACK.includes(e.key);
    if (!back && !ON.includes(e.key)) return;
    e.preventDefault();
    const at = pills.findIndex((p) => p.v === value),
      to = (at + (back ? pills.length - 1 : 1)) % pills.length;
    onPick(pills[to].v);
    // the pill is kept (keyed by its value): focus it now, before the re-render
    e.currentTarget.querySelectorAll<HTMLElement>('[role="radio"]')[to]?.focus();
  };
  return (
    <Tag
      class={props.class}
      id={props.id}
      role="radiogroup"
      aria-labelledby={props.labelledBy}
      aria-label={props.label}
      hidden={props.hidden}
      onKeyDown={step}
    >
      {props.lead}
      {pills.map((p) => {
        const on = p.v === value;
        return (
          // biome-ignore lint/a11y/useSemanticElements: a pill is the game's button art; a radio input can't wear it
          // biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-pressed besides aria-checked: style.css and the suite's contrast audit select the chosen pill by it
          <button
            key={p.v}
            type="button"
            role="radio"
            data-v={p.dataV}
            aria-label={p.label}
            title={p.title}
            aria-checked={on}
            aria-pressed={on}
            tabIndex={on ? 0 : -1}
            onClick={() => {
              if (!on) onPick(p.v);
            }}
          >
            {p.body}
          </button>
        );
      })}
    </Tag>
  );
}
