import asyncio, os
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1000})
        logs = []; pg.on('pageerror', lambda e: logs.append(str(e))); pg.on('console', lambda m: m.type == 'error' and logs.append(m.text))
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(800)
        await pg.set_input_files('#img-files', ['test_1001.png', 'test_30116.png']); await pg.wait_for_timeout(500)
        print(await pg.inner_text('#img-count'))
        print('imgs in results', await pg.evaluate("document.querySelectorAll('#results img').length"), 'roster imgs', await pg.evaluate("document.querySelectorAll('#roster img').length"))
        el = await pg.query_selector('#results article'); await el.screenshot(path='shot_card_av.png')
        await pg.click('.why'); await pg.wait_for_timeout(400)
        await pg.screenshot(path='shot_why.png')
        print((await pg.inner_text('#why-body'))[:600])
        await pg.click('#why-body .btn')
        pg2 = await b.new_page(viewport={'width': 390, 'height': 900}); await pg2.goto('file://' + os.getcwd() + '/dist/index.html'); await pg2.wait_for_timeout(600)
        await pg2.click('.why'); await pg2.wait_for_timeout(300); await pg2.screenshot(path='shot_why_phone.png')
        print('phone scrollWidth', await pg2.evaluate('document.documentElement.scrollWidth'))
        print(logs); await b.close()
asyncio.run(main())
