"""Render docs/RESEARCH.md from docs/RESEARCH.template.md and the computed values in research/results.json.

A placeholder {{KEY|SPEC}} is replaced by the value at KEY formatted with SPEC: a Python format spec ('.3f', ',.0f',
'.1%', '+.1f'), with a leading 'u' to print a negative with a typographic minus. Every number the document states that
the scripts compute comes from here; a placeholder without a value is an error, so the document cannot state a number
the data did not produce.

  uv run --group research research/render.py           write docs/RESEARCH.md
  uv run --group research research/render.py --check   exit 1 if docs/RESEARCH.md differs from the rendering"""
import json, os, re, sys
HERE: str = os.path.dirname(os.path.abspath(__file__)); ROOT: str = os.path.dirname(HERE)
TEMPLATE: str; OUT: str; RESULTS: str
TEMPLATE, OUT, RESULTS = (os.path.join(ROOT, 'docs', 'RESEARCH.template.md'), os.path.join(ROOT, 'docs', 'RESEARCH.md'),
                          os.path.join(HERE, 'results.json'))
PH: re.Pattern[str] = re.compile(r'\{\{([0-9]+\.[a-z0-9_]+)\|(u?[^}]+)\}\}')


def fmt(v: float | str, spec: str) -> str:
    u = spec.startswith('u'); out = format(v, spec[1:] if u else spec)
    return out.replace('-', '−') if u else out


def render(template: str, values: dict[str, float | str]) -> str:
    missing = sorted({k for k, _ in PH.findall(template) if k not in values})
    if missing: sys.exit('no computed value for: ' + ', '.join(missing))
    return PH.sub(lambda m: fmt(values[m.group(1)], m.group(2)), template)


def main() -> None:
    values = json.load(open(RESULTS))['values']
    text = render(open(TEMPLATE, encoding='utf-8').read(), values)
    if '--check' in sys.argv:
        same = os.path.exists(OUT) and open(OUT, encoding='utf-8').read() == text
        print('docs/RESEARCH.md matches the rendering' if same else 'docs/RESEARCH.md differs from the rendering'); sys.exit(0 if same else 1)
    open(OUT, 'w', encoding='utf-8').write(text); print('wrote docs/RESEARCH.md')


if __name__ == '__main__': main()
