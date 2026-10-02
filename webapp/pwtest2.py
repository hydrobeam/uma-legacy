import asyncio, os, time
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1000})
        logs = []; pg.on('pageerror', lambda e: logs.append(str(e))); pg.on('console', lambda m: m.type == 'error' and logs.append(m.text))
        await pg.goto('file://' + os.getcwd() + '/test_plain.html'); await pg.wait_for_timeout(600)
        for nm in ['Inari One', 'Tamamo Cross']: await pg.get_by_role('button', name=nm + ':').first.click()
        await pg.click('#o-sum')
        await pg.get_by_role('button', name="Special Week [Hopp'n♪Happy Heart]: owned").click()
        await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=120000)
        txt = await pg.inner_text('#results'); print(txt[:400])
        print(await pg.inner_text('#r-count'), '|', await pg.inner_text('#o-sum'))
        print('scrollWidth', await pg.evaluate('document.documentElement.scrollWidth')); print(logs)
        await b.close()
asyncio.run(main())
