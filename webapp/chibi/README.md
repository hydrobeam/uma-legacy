# The runner strips (`webapp/chibi/`)

While a search runs, the page shows the uma being planned running (`webapp/src/images.js` `runnerHTML`; a spinner
when a build has no strips). `./build.sh runners` (`chibi.py [all | ID ...] [--force]`) makes one strip
per roster uma, both eras: her mini body, hair, the shared mini face in her skin tone, her tail
(`chara_data.tail_model_id`) and the game's mini run motion. AssetStudio's CLI exports FBX, FBX2glTF makes glb, the
run clip is decoded in Python (dense and constant curves, bones by the CRC32 of their path), and
`webapp/chibi/render.mjs` renders `chibi.html` (three.js) in headless Chromium: flat colours with an outline, 18 frames
(0.6 s) at 128 px, cropped to her box over the loop, one AVIF. Exports are cached per uma in `cache/chibi/work/ID`
(delete one to redo it). About 3 s per uma; the 83 roster umas come to 4.9 MB, and a page loads one.

For when it breaks:
- **Pieces and joins.** Body `mbdy{id}_NN` (her lowest variant: seven umas have no `_00`), hair `mchr{id}_00_hair`,
  the shared face `mchr0001_00_face0` (texture `face0_{skin-1}`), tail `mtail{tail_model_id:04}_00` with
  `tex_mtail…_{id}_diff`. Skins join the body's bones by name; nodes whose parent is a body bone (Tail_Ctrl under Hip,
  the hair and ear chains under Head) move under it, and the face's rigid eye, brow, mouth and cheek meshes move from
  the face file's Head to the body's. Materials need their `sourceresources/...` prerequisites (the index's `d` column;
  each bundle also lists them itself in `m_Dependencies`, names with a `.a` suffix).
- **Coordinates.** FBX2glTF scales positions by 1/100; Unity's x mirror converts position (-x, y, z) and quaternion
  (x, -y, -z, w). The run clip has 20 samples of which the loop is 18 (frame 18 = frame 0); `run.json` carries `loop`.
- **Looks.** The blush texture has no alpha and is drawn as a multiply. The mouth is a 4 x 8 sheet read through one
  cell (`mouth=2`, the open happy shape). The clip leaves the tail still, so the tail is a spring chain pinned to her
  hip (`droop` 30°, `k` 260, `damp` 3.5, `g` 0.35), run 8 loops at 240 Hz with the last recorded; hair, skirt and ears
  sway on a sine at the step rate.
- **Umas not yet on Global** (1104–1106) take skin and tail from the JP `master.mdb`; their mini models already ship in
  the Global files.
- **Not drawn:** attachments (`chara_data.attachment_model_id`); accessories inside the hair model are.
- **On the page:** the loop planner runs one of the result loop's umas (kept while that loop is shown), the career
  page the trainee; before either, a random uma. Her strip loads when she is picked. It is set as an inline
  `background-image`, since a `url()` in a CSS custom property resolves against the stylesheet, not the page.
