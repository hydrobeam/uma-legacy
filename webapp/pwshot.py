import asyncio, os
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1000})
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(600)
        await pg.click('#t-preset'); await pg.select_option('#c-top','3')
        await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=300000)
        print((await pg.inner_text('#results'))[:300])
        el = await pg.query_selector('#results'); await el.screenshot(path='shot_deck_top.png')
        print(await pg.evaluate("getComputedStyle(document.getElementById('aff-panel')).display"))
        await b.close()
asyncio.run(main())
