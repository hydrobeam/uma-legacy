// A small document for node tests that render components (Preact) or run the el() editors: elements with children,
// attributes, properties, listeners and the simple selectors the editors use ('tag', '.cls', 'tag.cls'). Every id
// the page looks up is an element made on first use (a 'data-*' id is none: data.js reads the fake page's data), so a
// module's load-time lookups find something. Import fakepage.mjs first, then this.
class Node {
  constructor(type) {
    this.nodeType = type;
    this.childNodes = [];
    this.parentNode = null;
  }
  get firstChild() {
    return this.childNodes[0] || null;
  }
  contains(n) {
    for (let e = n; e; e = e.parentNode) if (e === this) return true;
    return false;
  }
  get parentElement() {
    return this.parentNode?.nodeType === 1 ? this.parentNode : null;
  }
  get nextSibling() {
    const p = this.parentNode;
    return p ? p.childNodes[p.childNodes.indexOf(this) + 1] || null : null;
  }
  // as a real DOM: `ref` (when given) must be a child, and a node moved here leaves its old place first
  insertBefore(n, ref) {
    if (ref != null && ref.parentNode !== this) throw new Error('insertBefore: the reference node is not a child');
    if (n === ref) return n;
    if (n.parentNode) n.parentNode.removeChild(n);
    if (ref == null) this.childNodes.push(n);
    else this.childNodes.splice(this.childNodes.indexOf(ref), 0, n);
    n.parentNode = this;
    return n;
  }
  appendChild(n) {
    return this.insertBefore(n, null);
  }
  append(...nodes) {
    for (const n of nodes) this.appendChild(typeof n === 'string' ? new Text(n) : n);
  }
  // as a real DOM: a node that is not a child throws (splice(-1, 1) would drop the last child instead)
  removeChild(n) {
    const at = this.childNodes.indexOf(n);
    if (at < 0) throw new Error('removeChild: the node is not a child');
    this.childNodes.splice(at, 1);
    n.parentNode = null;
    return n;
  }
  remove() {
    this.parentNode?.removeChild(this);
  }
  get textContent() {
    return this.nodeType === 3 ? this.data : this.childNodes.map((c) => c.textContent).join('');
  }
  set textContent(text) {
    if (this.nodeType === 3) this.data = text;
    else {
      for (const c of this.childNodes) c.parentNode = null;
      this.childNodes = text ? [new Text(String(text))] : [];
    }
  }
}
class Text extends Node {
  constructor(data) {
    super(3);
    this.data = data;
  }
}
class Element extends Node {
  constructor(name) {
    super(1);
    this.localName = name;
    this.attributes = {};
    this.listeners = {};
    this.style = { setProperty: () => undefined };
    this.dataset = {};
  }
  setAttribute(k, v) {
    this.attributes[k] = String(v);
  }
  getAttribute(k) {
    return this.attributes[k] ?? null;
  }
  removeAttribute(k) {
    delete this.attributes[k];
  }
  // a text box's select-all (the preset view's rename box calls it): nothing to show here
  select() {
    return undefined;
  }
  addEventListener(type, f) {
    this.listeners[type] = [...(this.listeners[type] || []), f];
  }
  removeEventListener(type, f) {
    this.listeners[type] = (this.listeners[type] || []).filter((x) => x !== f);
  }
  get classList() {
    return { contains: (c) => this.classes.includes(c) };
  }
  get classes() {
    return `${this.className || ''} ${this.attributes.class || ''}`.split(/\s+/).filter(Boolean);
  }
  matches(sel) {
    const [tag, ...cls] = sel.split('.');
    return (!tag || tag === this.localName) && cls.every((c) => this.classes.includes(c));
  }
  querySelectorAll(sel) {
    const out = [];
    const walk = (n) => {
      for (const c of n.childNodes)
        if (c.nodeType === 1) {
          if (c.matches(sel)) out.push(c);
          walk(c);
        }
    };
    walk(this);
    return out;
  }
  querySelector(sel) {
    return this.querySelectorAll(sel)[0] || null;
  }
  closest(sel) {
    for (let e = this; e; e = e.parentElement) if (e.matches(sel)) return e;
    return null;
  }
  getBoundingClientRect() {
    return { left: 0, top: 0, right: 0, bottom: 0, width: 0, height: 0 };
  }
  showModal() {
    this.open = true;
  }
  // as a real dialog: closing an open one fires `close` (at once here; a browser queues it)
  close() {
    if (!this.open) return;
    this.open = false;
    const e = { type: 'close', target: this, currentTarget: this };
    for (const f of this.listeners.close || []) f.call(this, e);
  }
  focus() {
    globalThis.document.activeElement = this;
  }
  // a click as Preact's listener and an el() `onclick` both take it
  click() {
    const e = {
      type: 'click',
      target: this,
      currentTarget: this,
      preventDefault: () => undefined,
      stopPropagation: () => undefined,
    };
    for (const f of this.listeners.click || []) f.call(this, e);
    this.onclick?.(e);
  }
}
const ids = new Map();
const body = new Element('body');
globalThis.document = {
  body,
  documentElement: Object.assign(new Element('html'), { className: 'game' }), // (the planner pages' class)
  activeElement: body,
  visibilityState: 'visible',
  createElement: (name) => new Element(name),
  createElementNS: (_, name) => new Element(name),
  createTextNode: (data) => new Text(data),
  getElementById: (id) => {
    if (id.startsWith('data-')) return null;
    if (!ids.has(id)) ids.set(id, Object.assign(new Element('div'), { id }));
    return ids.get(id);
  },
  querySelectorAll: () => [],
  querySelector: () => null,
  // the document's listeners: a test delivers an event to them with `dispatchToDocument` (a click bubbling up, say)
  listeners: {},
  addEventListener(type, f) {
    this.listeners[type] = [...(this.listeners[type] || []), f];
  },
  removeEventListener(type, f) {
    this.listeners[type] = (this.listeners[type] || []).filter((x) => x !== f);
  },
};
/** an event reaching the document's listeners (after the target's own, as a bubbling one does) */
export function dispatchToDocument(type, fields) {
  const e = { type, preventDefault: () => undefined, stopPropagation: () => undefined, ...fields };
  for (const f of globalThis.document.listeners[type] || []) f(e);
}
globalThis.Image = class extends Element {
  constructor() {
    super('img');
  }
};
globalThis.CSS = { escape: (s) => s };
globalThis.requestAnimationFrame = (f) => setTimeout(f, 0);
globalThis.cancelAnimationFrame = (id) => clearTimeout(id);
Object.assign(globalThis, { scrollX: 0, scrollY: 0, innerWidth: 1440, innerHeight: 900 });
globalThis.matchMedia = () => ({ matches: true, addEventListener: () => undefined });
globalThis.addEventListener = () => undefined;
export { Element };
