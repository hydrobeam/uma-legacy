import asyncio, os, time
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1400})
        logs = []; pg.on('pageerror', lambda e: logs.append(str(e))); pg.on('console', lambda m: m.type == 'error' and logs.append(m.text))
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(800)
        await pg.click('#t-preset')
        print(await pg.inner_text('#c-count'))
        for nm in ['Tamamo Cross', 'Inari One', 'Nakayama Festa', 'Ines Fujin', 'Gold City']:
            await pg.get_by_role('button', name=nm + ':').first.click()
        for nm in ['Sasami Anshinzawa SSR This Might Sting!', 'Daitaku Helios SSR Sunlit Angel ♪']:
            await pg.get_by_role('button', name=nm).first.click()
        print(await pg.inner_text('#c-count'))
        await pg.screenshot(path='shot_deck_panel.png', full_page=False)
        t=time.time(); await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=300000)
        print('took', round(time.time()-t,1))
        txt = await pg.inner_text('#results'); print(txt[:900])
        await pg.screenshot(path='shot_deck_result.png', full_page=False)
        pg2 = await b.new_page(viewport={'width': 390, 'height': 1600}); await pg2.goto('file://' + os.getcwd() + '/dist/index.html'); await pg2.wait_for_timeout(600)
        print('phone scrollWidth', await pg2.evaluate('document.documentElement.scrollWidth'))
        print(logs); await b.close()
asyncio.run(main())
