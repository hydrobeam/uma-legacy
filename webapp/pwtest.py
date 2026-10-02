import asyncio, sys, time
from playwright.async_api import async_playwright
async def main(fn, shot, width, dark):
    async with async_playwright() as p:
        b = await p.chromium.launch()
        pg = await b.new_page(viewport={'width': width, 'height': 1000}, color_scheme='dark' if dark else 'light')
        logs = []
        pg.on('console', lambda m: logs.append(f'{m.type}: {m.text}'))
        pg.on('pageerror', lambda e: logs.append(f'pageerror: {e}'))
        await pg.goto('file://' + fn)
        await pg.wait_for_timeout(800)
        await pg.screenshot(path=shot + '_initial.png', full_page=False)
        # exclude Inari and Tamamo, run
        for nm in ['Inari One', 'Tamamo Cross']:
            await pg.get_by_role('button', name=nm + ':').first.click()
        t = time.time()
        await pg.click('#run')
        await pg.wait_for_selector('.loop', timeout=120000)
        await pg.wait_for_function("!document.getElementById('run').disabled", timeout=120000)
        print('search took', round(time.time() - t, 1), 's')
        print((await pg.inner_text('#results'))[:700])
        await pg.screenshot(path=shot + '_result.png', full_page=False)
        sw = await pg.evaluate('document.documentElement.scrollWidth')
        print('scrollWidth', sw, 'viewport', width)
        print('\n'.join(logs[:20]))
        await b.close()
asyncio.run(main(*sys.argv[1:3], int(sys.argv[3]), sys.argv[4] == 'dark'))
