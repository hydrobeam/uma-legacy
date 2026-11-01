// The node popup (game style): one dialog for the tree's places and the pool's veterans. Its title, body and actions
// are built by career.js's el() fillers (bridged until step 8 ports them), again whenever the page redraws while it is
// open; closed, it keeps what it showed.
import { render } from 'preact';
import { useRef } from 'preact/hooks';
import { signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { must } from '../../must.ts';
import { Dialog } from '../../ui/Dialog.tsx';
import { rebuild } from '../../ui/rebuild.ts';

// the node on show: {kind: 't'} the trainee, {kind: 'p', k} parent place k, {kind: 'g', k, j} parent k's parent j,
// {kind: 'v', id} a veteran of the pool
export type NodeSpec = { kind: 't' | 'p' | 'g' | 'v'; k?: number; j?: number; id?: string };
// what career.js hands the popup: its redraws (`drawn`); `fill`, which builds a node's body and actions and returns its
// title ('' when the node is gone: a removed veteran, a grandparent whose parent place emptied); `closed`, which puts
// the focus back on the node's button
type NodeProps = {
  drawn: ReadonlySignal<number>;
  fill: (spec: NodeSpec, body: HTMLElement, acts: HTMLElement) => string;
  closed: (spec: NodeSpec) => void;
};
const showing = signal<NodeSpec | null>(null);
export const openNode = (spec: NodeSpec) => {
  showing.value = spec;
};
export const closeNode = () => {
  showing.value = null;
};
function NodePopup({ drawn, fill, closed }: NodeProps) {
  const spec = showing.value,
    last = useRef<NodeSpec | null>(null);
  void drawn.value;
  // her content, made again on each render while open (a ref runs before the Dialog shows it); the actions go in the
  // footer beside Close
  const refill = (body: HTMLDivElement | null) => {
    if (!body || !spec) return;
    last.current = spec;
    const dialog = must(body.closest('dialog'), 'the body is in the dialog'),
      foot = must(body.nextElementSibling, 'the footer follows the body');
    rebuild(dialog, () => {
      const acts = document.createElement('div');
      body.textContent = '';
      const title = fill(spec, body, acts);
      if (!title) {
        dialog.close();
        return;
      }
      must(body.previousElementSibling, 'the title precedes the body').textContent = title;
      for (const e of foot.querySelectorAll('.nact')) e.remove();
      for (const b of [...acts.children]) {
        b.classList.add('nact');
        foot.appendChild(b);
      }
    });
  };
  const onClose = () => {
    showing.value = null;
    if (last.current) closed(last.current);
  };
  return (
    <Dialog open={!!spec} onClose={onClose} class="gmdlg nodedlg" id="node-dlg" aria-labelledby="node-title">
      {/* biome-ignore lint/a11y/useHeadingContent: its text is the filler's title, written before the dialog shows */}
      <h3 class="gmdhdr" id="node-title" />
      <div class="gmdbody" id="node-body" ref={refill} />
      <div class="gmdfoot">
        <button type="button" class="btn gmclose" onClick={closeNode}>
          Close
        </button>
      </div>
    </Dialog>
  );
}
// the popup in a host of its own at the end of the page (render() owns its container's children)
export function mountNode(props: NodeProps) {
  const host = document.createElement('div');
  document.body.appendChild(host);
  render(<NodePopup {...props} />, host);
}
