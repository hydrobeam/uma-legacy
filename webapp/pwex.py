import asyncio, os
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 900})
        errs=[]; pg.on('pageerror', lambda e: errs.append(str(e)))
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(800)
        await pg.click('#ex-open'); t = await pg.input_value('#ex-text'); print(t[:700]); print(len(t))
        await pg.click('#ex-copy'); print(await pg.inner_text('#ex-msg'))
        await pg.fill('#ex-text', t.replace('"borrow": true','"borrow": false')); await pg.click('#ex-load'); print(await pg.inner_text('#ex-msg'), await pg.is_checked('#c-borrow'))
        print(errs); await b.close()
asyncio.run(main())
