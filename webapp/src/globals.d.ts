// Declarations for the type checker only (tsconfig.json; nothing here is bundled): what the page adds to the browser's
// globals and elements, and the element type its selector queries return.

interface Window {
  /** build.py's fingerprinted data scripts: each embedded JSON value, keyed by the id of the element that used to hold it */
  __LLP_DATA?: Record<string, unknown>;
}

interface HTMLElement {
  /** a roster or card tile: redraws it from the current state */
  _paint?: () => void;
  /** a roster, card or uma-picker tile: its name in lower case, which the search box matches */
  _searchName?: string;
  /** a roster tile: its outfit's label in lower case ('' for none), which the search box also matches */
  _outfitLabel?: string;
  /** an uma-picker button: her name in lower case */
  _nm?: string;
}

// The page's class and attribute selectors find HTML elements (its inline SVG icons are never queried), so a query or a
// `closest` by selector returns an HTMLElement; one by tag name keeps the tag's own element type (those overloads repeat
// lib.dom's: a merged declaration's overloads are tried before the library's).
interface ParentNode {
  querySelector<K extends keyof HTMLElementTagNameMap>(selectors: K): HTMLElementTagNameMap[K] | null;
  querySelector<K extends keyof SVGElementTagNameMap>(selectors: K): SVGElementTagNameMap[K] | null;
  querySelector<E extends Element = HTMLElement>(selectors: string): E | null;
  querySelectorAll<K extends keyof HTMLElementTagNameMap>(selectors: K): NodeListOf<HTMLElementTagNameMap[K]>;
  querySelectorAll<K extends keyof SVGElementTagNameMap>(selectors: K): NodeListOf<SVGElementTagNameMap[K]>;
  querySelectorAll<E extends Element = HTMLElement>(selectors: string): NodeListOf<E>;
}
interface Element {
  closest<K extends keyof HTMLElementTagNameMap>(selector: K): HTMLElementTagNameMap[K] | null;
  closest<E extends Element = HTMLElement>(selectors: string): E | null;
}

/**
 * An event whose target is known to be a T (by default an element): the listener hears events from such targets only
 * (a click or a key press lands on an element; a change event on the input that changed). lib.dom types every event's
 * target as a bare EventTarget.
 */
type TargetedEvent<T extends EventTarget = Element, E extends Event = Event> = E & { target: T };
