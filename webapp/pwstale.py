import asyncio, os
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 900})
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(800)
        print('stale at load', await pg.evaluate("!!document.querySelector('.stale')"))
        await pg.get_by_role('button', name='Daitaku Helios SSR Sunlit Angel ♪').first.click()
        print('stale after change', await pg.evaluate("!!document.querySelector('.stale')"))
        await pg.screenshot(path='shot_stale.png', clip={'x':470,'y':0,'width':810,'height':300})
        await b.close()
asyncio.run(main())
