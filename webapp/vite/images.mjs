// The game's images (webapp/img, from images.py and chibi/chibi.py), which the page loads by id at runtime: copied to
// assets/img.<hash>/<kind>/<id>.<ext>, the hash taken over every file's name and content, so any change gives the whole
// set a new folder and the folder is cached a year like the other assets. The page gets the catalog (`catalog`: the
// keys, each kind's extension, the folder). The UI sprites the stylesheet draws the game's buttons with are also set as
// CSS variables (--btn-w, --spark-white, ...) at the end of src/style.css, so they show at the first paint; a sprite the
// build lacks is left unset (its rules fall back to plain styles), and Vite fingerprints the ones it has.
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

const KINDS = ['uma', 'mob', 'outfit', 'vet', 'card', 'skill', 'race', 'scen', 'scenart', 'ui', 'run'];
// biome-ignore format: a list, filled to the line
const SPRITES = [
  'gauge_bond', 'friend_frame', 'spark_white', 'spark_blue', 'spark_pink', 'spark_green', 'spark_dot',
  'spark_dot_gold', 'tag_blue', 'tag_pink', 'tag_green', 'skill_white', 'skill_gold', 'skill_unique', 'skill_pink',
  'pill', 'star_on', 'star_off', 'spark_white_dark', 'skill_white_dark', 'star_off_dark', 'enh_umas', 'enh_cards',
  'enh_vets', 'enh_umas_dark', 'enh_cards_dark', 'enh_vets_dark', 'btn_w', 'btn_g', 'pill_w', 'pill_g', 'chk_off',
  'chk_on', 'btn_w_dark', 'pill_w_dark', 'chk_off_dark', 'tog_off', 'tog_on', 'tog_off_dark', 'slot_frame',
  'slot_empty', 'sched_badge', 'plus', 'slot_frame_dark', 'slot_empty_dark', 'close',
];

/** The images in `dir` (AVIF, UI sprites lossless WebP; one extension per kind): their keys ('uma:1001', ...), each
 * kind's extension, the fingerprinted folder, and the files to copy. */
export function imageCatalog(dir) {
  const keys = [],
    ext = {},
    files = [],
    hash = createHash('sha256');
  for (const kind of KINDS) {
    const kindDir = path.join(dir, kind);
    const names = fs.existsSync(kindDir) ? fs.readdirSync(kindDir).sort() : [];
    for (const name of names) {
      const { name: stem, ext: dotExt } = path.parse(name);
      if (dotExt !== '.avif' && dotExt !== '.webp') continue;
      ext[kind] ??= dotExt.slice(1);
      if (ext[kind] !== dotExt.slice(1))
        throw new Error(`img/${kind} mixes formats (${name}); rerun ./build.sh images`);
      const source = path.join(kindDir, name);
      hash.update(`${kind}/${name}`).update(fs.readFileSync(source));
      files.push({ kind, name, source });
      keys.push(`${kind}:${stem}`);
    }
  }
  return { keys, ext, base: `assets/img.${hash.digest('hex').slice(0, 10)}/`, files };
}

/** The plugin: emits the catalog's files, and appends the sprites' variables to `stylesheet` (an absolute path). */
export function images(catalog, imgDir, stylesheet) {
  const present = new Set(catalog.keys);
  const spriteRule = SPRITES.filter((k) => present.has(`ui:${k}`))
    .map(
      (k) =>
        `--${k.replaceAll('_', '-')}: url(${path.relative(path.dirname(stylesheet), imgDir)}/ui/${k}.${catalog.ext.ui});`,
    )
    .join(' ');
  return {
    name: 'llp-images',
    enforce: 'pre', // the rule is added before Vite reads the stylesheet's url()s
    transform(code, id) {
      if (id !== stylesheet || !spriteRule) return null;
      return { code: `${code}\n:root { ${spriteRule} }\n`, map: null };
    },
    generateBundle() {
      for (const { kind, name, source } of catalog.files)
        this.emitFile({ type: 'asset', fileName: `${catalog.base}${kind}/${name}`, source: fs.readFileSync(source) });
    },
  };
}
